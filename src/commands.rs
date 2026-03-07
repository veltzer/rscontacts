use google_people1::FieldMask;

use crate::helpers::*;

const DEFAULT_CONFIG: &str = r#"# rscontacts configuration

[check-all]
# List of checks to skip when running check-all.
# All checks not listed here will run by default.
# Uncomment any check name to skip it.
skip = [
    # "check-contact-given-name-regexp",
    # "check-contact-family-name-regexp",
    # "check-contact-suffix-regexp",
    # "check-contact-displayname-duplicate",
    # "check-contact-no-label",
    # "check-contact-email",
    # "check-contact-email-duplicate",
    # "check-contact-label-nophone",
    # "check-contact-label-space",
    # "check-contact-label-camelcase",
    # "check-phone-countrycode",
    # "check-phone-format",
    # "check-phone-label-missing",
    # "check-phone-label-english",
    # "check-phone-duplicate",
]

# Allow regex for given names. Contacts whose given name does NOT match
# this pattern will be flagged by check-contact-given-name-regexp.
# Single uppercase letter or uppercase followed by lowercase letters.
[check-contact-given-name-regexp]
allow = '^[A-Z][a-z]*$'

# Allow regex for family names. Contacts whose family name does NOT match
# this pattern will be flagged by check-contact-family-name-regexp.
# First letter uppercase, rest lowercase. Hyphenated names allowed.
[check-contact-family-name-regexp]
allow = '^[A-Z][a-z]+(-[A-Z][a-z]+)*$'

# Allow regex for suffixes. Contacts whose suffix does NOT match
# this pattern will be flagged by check-contact-suffix-regexp.
# Default (if not configured): numeric, no leading zero (^[1-9]\d*$).
# [check-contact-suffix-regexp]
# allow = '^[1-9]\d*$'
"#;

pub fn cmd_init_config(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    if path.exists() && !force {
        eprintln!("Config file already exists at {}", path.display());
        eprintln!("Use --force to overwrite.");
        std::process::exit(1);
    }
    std::fs::write(&path, DEFAULT_CONFIG)?;
    eprintln!("Created config file at {}", path.display());
    Ok(())
}

pub async fn cmd_auth(no_browser: bool, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    if force {
        let cache = token_cache_path();
        if cache.exists() {
            std::fs::remove_file(&cache)?;
            eprintln!("Removed cached token at {}", cache.display());
        }
    }
    let secret = yup_oauth2::read_application_secret(credentials_path()).await?;

    let mut builder = yup_oauth2::InstalledFlowAuthenticator::builder(
        secret,
        yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk(token_cache_path());

    if !no_browser {
        builder = builder.flow_delegate(Box::new(BrowserFlowDelegate));
    }

    let auth = builder.build().await?;

    // Actually request a token so it gets persisted
    let scopes = &["https://www.googleapis.com/auth/contacts"];
    let _token = auth.token(scopes).await?;

    eprintln!("Authentication successful. Token cached to {}", token_cache_path().display());
    Ok(())
}

pub async fn cmd_list(emails: bool, labels: bool, starred: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let mut fields = vec!["names", "organizations", "phoneNumbers"];
    if emails { fields.push("emailAddresses"); }
    if labels || starred { fields.push("memberships"); }
    let contacts = fetch_all_contacts(&hub, &fields).await?;

    let contacts: Vec<_> = if starred {
        contacts.into_iter().filter(|p| is_starred(p)).collect()
    } else {
        contacts
    };

    // Build group resource name -> display name map when showing labels
    let group_names: std::collections::HashMap<String, String> = if labels {
        let all_groups = fetch_all_contact_groups(&hub).await?;
        all_groups.iter()
            .filter_map(|g| {
                let rn = g.resource_name.as_deref()?;
                let name = g.name.as_deref()?;
                Some((rn.to_string(), name.to_string()))
            })
            .collect()
    } else {
        std::collections::HashMap::new()
    };

    for person in &contacts {
        let name = person_display_name(person);

        let phone = person
            .phone_numbers
            .as_ref()
            .and_then(|phones| phones.first())
            .and_then(|p| p.value.as_deref())
            .unwrap_or("");

        let mut parts = vec![name];

        if emails {
            let email = person_email(person);
            if !email.is_empty() {
                parts.push(email.to_string());
            }
        }

        if !phone.is_empty() {
            parts.push(phone.to_string());
        }

        if labels {
            let contact_labels: Vec<&str> = person.memberships.as_ref()
                .map(|memberships| {
                    memberships.iter().filter_map(|m| {
                        let rn = m.contact_group_membership.as_ref()?
                            .contact_group_resource_name.as_deref()?;
                        if rn == "contactGroups/myContacts" { return None; }
                        group_names.get(rn).map(|s| s.as_str())
                    }).collect()
                })
                .unwrap_or_default();
            if !contact_labels.is_empty() {
                parts.push(format!("[{}]", contact_labels.join(", ")));
            }
        }

        println!("{}", parts.join(" | "));
    }

    Ok(())
}

async fn check_phone_issues<P, T>(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    predicate: P,
    transform: T,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>>
where
    P: Fn(&str) -> bool,
    T: Fn(&str) -> String + Clone,
{
    let filtered: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        p.phone_numbers.as_ref().is_some_and(|nums| nums.iter().any(|pn| {
            pn.value.as_deref().is_some_and(|v| predicate(v))
        }))
    }).collect();

    if !quiet {
        if !filtered.is_empty() {
            if let Some(header) = header {
                println!("=== {} ({}) ===", header, filtered.len());
            }
        }

        for person in &filtered {
            let name = person_display_name(person);

            if let Some(nums) = &person.phone_numbers {
                for pn in nums {
                    if let Some(val) = pn.value.as_deref() {
                        if predicate(val) {
                            let fixed = transform(val);
                            print_phone_fix(&name, val, &fixed, fix, dry_run, prefix);
                        }
                    }
                }
            }

            if fix && !dry_run {
                let transform = transform.clone();
                update_phone_numbers(hub, person, |val| {
                    if predicate(val) {
                        Some(transform(val))
                    } else {
                        None
                    }
                }).await?;
            }
        }

        if !filtered.is_empty() && header.is_some() {
            println!();
        }
    }

    Ok(filtered.len())
}

pub async fn cmd_check_contact_given_name_regexp(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
    let config = load_config();
    let (user_groups_owned, label_names) = if fix {
        let ug: Vec<(String, String)> = all_groups.iter()
            .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
            .filter_map(|g| {
                let name = g.name.as_deref()?;
                let rn = g.resource_name.as_deref()?;
                Some((name.to_string(), rn.to_string()))
            })
            .collect();
        let ln: Vec<String> = ug.iter().map(|(name, _)| name.clone()).collect();
        (ug, ln)
    } else {
        (vec![], vec![])
    };
    let user_groups: Vec<(&str, &str)> = user_groups_owned.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();
    check_given_name_regexp(&hub, &contacts, &group_names, &config.check_contact_given_name_regexp, fix, dry_run, "", None, false, &user_groups, &label_names).await?;
    Ok(())
}

async fn check_given_name_regexp(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    group_names: &std::collections::HashMap<String, String>,
    config: &crate::helpers::NameRegexpConfig,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
    user_groups: &[(&str, &str)],
    label_names: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = match &config.allow {
        Some(p) => p,
        None => {
            if !quiet {
                eprintln!("No given name allow regex configured in config.toml. Set [check-contact-given-name-regexp] allow = \"...\"");
            }
            return Ok(0);
        }
    };

    let re = match regex::Regex::new(pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("Warning: invalid regex \"{}\": {}", pattern, e);
            return Ok(0);
        }
    };

    let mut count = 0;
    for person in contacts {
        let given = person.names.as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.given_name.as_deref())
            .unwrap_or("");
        if given.is_empty() {
            continue;
        }

        if !re.is_match(given) {
            if !quiet {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let display = person_display_name(person);
                let family = person.names.as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.family_name.as_deref())
                    .unwrap_or("");
                let suffix = person.names.as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.honorific_suffix.as_deref())
                    .unwrap_or("");
                let labels = person_labels(person, group_names);
                let labels_str = if labels.is_empty() {
                    String::new()
                } else {
                    format!(", labels: [{}]", labels.join(", "))
                };
                println!("{}{} (given: \"{}\", family: \"{}\", suffix: \"{}\"{})", prefix, display, given, family, suffix, labels_str);

                if fix && !dry_run {
                    interactive_given_name_fix(hub, person, given, user_groups, label_names).await?;
                }
            }
            count += 1;
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

async fn interactive_given_name_fix(
    hub: &HubType,
    person: &google_people1::api::Person,
    given: &str,
    user_groups: &[(&str, &str)],
    label_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;
    let family = person.names.as_ref()
        .and_then(|names| names.first())
        .and_then(|n| n.family_name.as_deref())
        .unwrap_or("");

    // Try splitting given name alone, given+family, or given+suffix
    let suffix = person.names.as_ref()
        .and_then(|names| names.first())
        .and_then(|n| n.honorific_suffix.as_deref())
        .unwrap_or("");
    let split_source = if split_alpha_numeric(given).is_some() {
        Some(given.to_string())
    } else if !family.is_empty() && split_alpha_numeric(&format!("{}{}", given, family)).is_some() {
        Some(format!("{}{}", given, family))
    } else if !suffix.is_empty() && split_alpha_numeric(&format!("{}{}", given, suffix)).is_some() {
        Some(format!("{}{}", given, suffix))
    } else {
        None
    };

    match prompt_given_name_fix_action(given, family, split_source.as_deref())? {
        'p' => {
            // Split: "Mike2" or "P"+"51" -> given_name="Mike"/"P", suffix="2"/"51"
            let source = split_source.as_deref().expect("split option only available when splittable");
            let (alpha, numeric) = split_alpha_numeric(source).expect("split option only available when splittable");
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.given_name = Some(alpha.to_string());
                    first.family_name = None;
                    first.honorific_suffix = Some(numeric.to_string());
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Split: given name=\"{}\", suffix=\"{}\"", alpha, numeric);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'w' => {
            // Swap: set given_name = family_name, clear family_name
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.given_name = first.family_name.take();
                    first.family_name = None;
                    let new_given = first.given_name.as_deref().unwrap_or("");
                    first.unstructured_name = Some(new_given.to_string());
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Swapped: given name is now \"{}\"", family);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'c' => {
            // Move given name to company (organization name), clear name fields
            let mut updated = person.clone();
            let company = given.to_string();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.given_name = None;
                    first.family_name = None;
                    first.unstructured_name = None;
                }
            }
            let org = google_people1::api::Organization {
                name: Some(company.clone()),
                ..Default::default()
            };
            updated.organizations = Some(vec![org]);
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names", "organizations"]))
                .doit()
                .await?;
            eprintln!("  Moved to company: \"{}\"", company);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'r' => {
            let new_name = prompt_new_name(given)?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.given_name = Some(new_name.clone());
                    first.family_name = None;
                    first.unstructured_name = Some(new_name.clone());
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Renamed to \"{}\"", new_name);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'u' => {
            // Add/set suffix
            let new_suffix = prompt_new_name("suffix")?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.honorific_suffix = Some(new_suffix.clone());
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Set suffix to \"{}\"", new_suffix);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'l' => {
            if let Some(group_rn) = prompt_label_autocomplete(hub, label_names, user_groups).await? {
                let req = google_people1::api::ModifyContactGroupMembersRequest {
                    resource_names_to_add: Some(vec![resource_name.to_string()]),
                    resource_names_to_remove: None,
                };
                hub.contact_groups().members_modify(req, &group_rn).doit().await?;
                eprintln!("  Assigned label.");
                tokio::time::sleep(MUTATE_DELAY).await;
            } else {
                eprintln!("  Skipped.");
            }
        }
        'd' => {
            hub.people().delete_contact(resource_name).doit().await?;
            eprintln!("  Deleted.");
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        's' => {
            eprintln!("  Skipped.");
        }
        _ => unreachable!(),
    }
    Ok(())
}


async fn interactive_family_name_fix(
    hub: &HubType,
    person: &google_people1::api::Person,
    family: &str,
    user_groups: &[(&str, &str)],
    label_names: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    match prompt_family_name_fix_action()? {
        'x' => {
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.family_name = None;
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Removed family name \"{}\"", family);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'r' => {
            let new_name = prompt_new_name(family)?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.family_name = Some(new_name.clone());
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Renamed family name to \"{}\"", new_name);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'u' => {
            let new_suffix = prompt_new_name("suffix")?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.honorific_suffix = Some(new_suffix.clone());
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Set suffix to \"{}\"", new_suffix);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'l' => {
            if let Some(group_rn) = prompt_label_autocomplete(hub, label_names, user_groups).await? {
                let req = google_people1::api::ModifyContactGroupMembersRequest {
                    resource_names_to_add: Some(vec![resource_name.to_string()]),
                    resource_names_to_remove: None,
                };
                hub.contact_groups().members_modify(req, &group_rn).doit().await?;
                eprintln!("  Assigned label.");
                tokio::time::sleep(MUTATE_DELAY).await;
            } else {
                eprintln!("  Skipped.");
            }
        }
        'd' => {
            hub.people().delete_contact(resource_name).doit().await?;
            eprintln!("  Deleted.");
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        's' => {
            eprintln!("  Skipped.");
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub async fn cmd_check_contact_suffix_regexp(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
    let config = load_config();
    check_suffix_regexp(&hub, &contacts, &group_names, &config.check_contact_suffix_regexp, fix, dry_run, "", None, false).await?;
    Ok(())
}

const DEFAULT_SUFFIX_REGEX: &str = r"^[1-9]\d*$";

async fn check_suffix_regexp(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    group_names: &std::collections::HashMap<String, String>,
    config: &crate::helpers::NameRegexpConfig,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let default_pattern = DEFAULT_SUFFIX_REGEX.to_string();
    let pattern = config.allow.as_ref().unwrap_or(&default_pattern);

    let re = match regex::Regex::new(pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("Warning: invalid regex \"{}\": {}", pattern, e);
            return Ok(0);
        }
    };

    let mut count = 0;
    for person in contacts {
        let suffix = person.names.as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.honorific_suffix.as_deref())
            .unwrap_or("");
        if suffix.is_empty() {
            continue;
        }

        if !re.is_match(suffix) {
            if !quiet {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let display = person_display_name(person);
                let labels = person_labels(person, group_names);
                let labels_str = if labels.is_empty() {
                    String::new()
                } else {
                    format!(", labels: [{}]", labels.join(", "))
                };
                println!("{}{} (suffix: \"{}\"{})", prefix, display, suffix, labels_str);

                if fix && !dry_run {
                    interactive_name_fix(hub, person, &display).await?;
                }
            }
            count += 1;
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_family_name_regexp(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
    let config = load_config();
    let (user_groups_owned, label_names) = if fix {
        let ug: Vec<(String, String)> = all_groups.iter()
            .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
            .filter_map(|g| {
                let name = g.name.as_deref()?;
                let rn = g.resource_name.as_deref()?;
                Some((name.to_string(), rn.to_string()))
            })
            .collect();
        let ln: Vec<String> = ug.iter().map(|(name, _)| name.clone()).collect();
        (ug, ln)
    } else {
        (vec![], vec![])
    };
    let user_groups: Vec<(&str, &str)> = user_groups_owned.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();
    check_family_name_regexp(&hub, &contacts, &group_names, &config.check_contact_family_name_regexp, fix, dry_run, "", None, false, &user_groups, &label_names).await?;
    Ok(())
}

async fn check_family_name_regexp(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    group_names: &std::collections::HashMap<String, String>,
    config: &crate::helpers::NameRegexpConfig,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
    user_groups: &[(&str, &str)],
    label_names: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = match &config.allow {
        Some(p) => p,
        None => {
            if !quiet {
                eprintln!("No family name allow regex configured in config.toml. Set [check-contact-family-name-regexp] allow = \"...\"");
            }
            return Ok(0);
        }
    };

    let re = match regex::Regex::new(pattern) {
        Ok(re) => re,
        Err(e) => {
            eprintln!("Warning: invalid regex \"{}\": {}", pattern, e);
            return Ok(0);
        }
    };

    let mut count = 0;
    for person in contacts {
        let family = person.names.as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.family_name.as_deref())
            .unwrap_or("");
        if family.is_empty() {
            continue;
        }

        if !re.is_match(family) {
            if !quiet {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let display = person_display_name(person);
                let given = person.names.as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.given_name.as_deref())
                    .unwrap_or("");
                let labels = person_labels(person, group_names);
                let labels_str = if labels.is_empty() {
                    String::new()
                } else {
                    format!(", labels: [{}]", labels.join(", "))
                };
                println!("{}{} (given: \"{}\", family: \"{}\"{})", prefix, display, given, family, labels_str);

                if fix && !dry_run {
                    interactive_family_name_fix(hub, person, family, user_groups, label_names).await?;
                }
            }
            count += 1;
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_displayname_duplicate(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers"]).await?;
    check_name_duplicate(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_name_duplicate(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut name_groups: std::collections::HashMap<String, Vec<&google_people1::api::Person>> =
        std::collections::HashMap::new();
    for person in contacts {
        let name = person_name(person);
        if !name.is_empty() {
            name_groups.entry(name.to_string()).or_default().push(person);
        }
    }

    let mut duplicates: Vec<(&str, &[&google_people1::api::Person])> = name_groups
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(name, group)| (name.as_str(), group.as_slice()))
        .collect();
    duplicates.sort_by_key(|(name, _)| *name);

    let count: usize = duplicates.iter().map(|(_, group)| group.len()).sum();

    if !quiet && !duplicates.is_empty() {
        if let Some(header) = header {
            println!("=== {} ({}) ===", header, count);
        }

        for (name, group) in &duplicates {
            println!("{}\"{}\" ({} contacts):", prefix, name, group.len());
            for person in *group {
                let email = person_email(person);
                let phone = person.phone_numbers.as_ref()
                    .and_then(|nums| nums.first())
                    .and_then(|p| p.value.as_deref())
                    .unwrap_or("");
                let mut info = vec![];
                if !email.is_empty() { info.push(email.to_string()); }
                if !phone.is_empty() { info.push(phone.to_string()); }
                if info.is_empty() {
                    println!("{}  - {}", prefix, name);
                } else {
                    println!("{}  - {} ({})", prefix, name, info.join(", "));
                }
            }

            if fix && !dry_run {
                // Ask to rename all but the first
                for person in &group[1..] {
                    let display = person_display_name(person);
                    let email = person_email(person);
                    if !email.is_empty() {
                        eprintln!("{}  Fix duplicate: {} ({})", prefix, display, email);
                    } else {
                        eprintln!("{}  Fix duplicate: {}", prefix, display);
                    }
                    interactive_name_fix(hub, person, &display).await?;
                }
            }
        }

        if header.is_some() {
            println!();
        }
    }

    Ok(count)
}

pub async fn cmd_check_phone_countrycode(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers"]).await?;
    let country = country.to_string();
    check_phone_issues(
        &hub, &contacts,
        |v| is_fixable_phone(v) && !has_country_code(v),
        move |v| add_country_code(v, &country),
        fix, dry_run, "", None, false,
    ).await?;
    Ok(())
}

pub async fn cmd_check_phone_format(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers"]).await?;
    let country_owned = country.to_string();
    check_phone_issues(
        &hub, &contacts,
        |v| is_fixable_phone(v) && !is_correct_phone_format(v),
        move |v| fix_phone_format(v, &country_owned),
        fix, dry_run, "", None, false,
    ).await?;
    Ok(())
}

async fn check_duplicate_phones(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            let values: Vec<&str> = nums.iter().filter_map(|pn| pn.value.as_deref()).collect();
            let dupes = find_duplicates(&values);
            if !dupes.is_empty() {
                if !quiet {
                    if count == 0 {
                        if let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    }
                    let name = person_display_name(person);
                    for phone in &dupes {
                        println!("{}{} | {}", prefix, name, phone);
                    }
                }
                count += dupes.len();

                if fix && !dry_run && !quiet {
                    let name = person_display_name(person);
                    if prompt_yes_no(&format!("Remove duplicate \"{}\" from {}?", dupes.join(", "), name))? {
                        remove_duplicate_phones(hub, person).await?;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

async fn check_email(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            let has_issue = emails.iter().any(|e| {
                e.value.as_deref().is_some_and(|v| !is_valid_email(v) || v != v.to_lowercase().as_str())
            });
            if !has_issue {
                continue;
            }

            if !quiet {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let name = person_display_name(person);
                for email in emails {
                    if let Some(val) = email.value.as_deref() {
                        if !is_valid_email(val) {
                            println!("{}{} | {} (invalid)", prefix, name, val);
                        } else if val != val.to_lowercase().as_str() {
                            if fix || dry_run {
                                println!("{}{} | {} -> {}", prefix, name, val, val.to_lowercase());
                            } else {
                                println!("{}{} | {} (uppercase)", prefix, name, val);
                            }
                        }
                    }
                }
            }

            let issue_count = emails.iter().filter(|e| {
                e.value.as_deref().is_some_and(|v| !is_valid_email(v) || v != v.to_lowercase().as_str())
            }).count();
            count += issue_count;

            if fix && !dry_run && !quiet {
                let has_caps = emails.iter().any(|e| {
                    e.value.as_deref().is_some_and(|v| is_valid_email(v) && v != v.to_lowercase().as_str())
                });
                if has_caps {
                    let name = person_display_name(person);
                    let resource_name = person
                        .resource_name
                        .as_deref()
                        .ok_or("Contact missing resource name")?;
                    let mut updated = person.clone();
                    if let Some(ref mut ems) = updated.email_addresses {
                        for e in ems.iter_mut() {
                            if let Some(ref val) = e.value {
                                e.value = Some(val.to_lowercase());
                            }
                        }
                    }
                    hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
                        .doit()
                        .await?;
                    eprintln!("  Lowercased emails for {}", name);
                    tokio::time::sleep(MUTATE_DELAY).await;
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

async fn check_no_label(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
    user_groups: &[(&str, &str)],
    label_names: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if !has_user_label(person) {
            if !quiet {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let name = person_display_name(person);
                let email = person_email(person);
                print_name_with_email(&name, email, prefix);
            }
            count += 1;

            if fix && !quiet {
                println!("{}", "=".repeat(60));
                print_person_details(person, None);
                println!("{}", "-".repeat(60));

                if dry_run {
                    eprintln!("(dry-run) would prompt for action\n");
                    continue;
                }
                let resource_name = match person.resource_name.as_deref() {
                    Some(rn) => rn,
                    None => continue,
                };
                use std::io::Write;
                loop {
                    eprint!("[l]abel / [d]elete / [s]kip: ");
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('l') => {
                            print_person_details(person, None);
                            println!("{}", "-".repeat(60));
                            if let Some(group_rn) = prompt_label_autocomplete(hub, label_names, user_groups).await? {
                                let req = google_people1::api::ModifyContactGroupMembersRequest {
                                    resource_names_to_add: Some(vec![resource_name.to_string()]),
                                    resource_names_to_remove: None,
                                };
                                hub.contact_groups().members_modify(req, &group_rn).doit().await?;
                                eprintln!("  Assigned label.");
                                tokio::time::sleep(MUTATE_DELAY).await;
                            } else {
                                eprintln!("  Skipped.");
                            }
                            break;
                        }
                        Some('d') => {
                            if prompt_yes_no(&format!("Delete {}?", person_display_name(person)))? {
                                hub.people().delete_contact(resource_name).doit().await?;
                                eprintln!("  Deleted.");
                                tokio::time::sleep(MUTATE_DELAY).await;
                            } else {
                                eprintln!("  Skipped.");
                            }
                            break;
                        }
                        Some('s') => {
                            eprintln!("  Skipped.");
                            break;
                        }
                        _ => eprintln!("  Invalid choice. Enter l, d, or s."),
                    }
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

async fn remove_duplicate_phones(hub: &HubType, person: &google_people1::api::Person) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut nums) = updated.phone_numbers {
        let mut seen = std::collections::HashSet::new();
        nums.retain(|pn| {
            let val = pn.value.as_deref().unwrap_or("");
            seen.insert(val.to_string())
        });
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
        .doit()
        .await?;
    eprintln!("  Removed duplicates for {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

pub async fn cmd_check_phone_duplicate(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers"]).await?;
    check_duplicate_phones(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_email(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses"]).await?;
    check_email(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_duplicate_emails(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            let values: Vec<&str> = emails.iter().filter_map(|e| e.value.as_deref()).collect();
            let dupes = find_duplicates(&values);
            if !dupes.is_empty() {
                if !quiet {
                    if count == 0 {
                        if let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    }
                    let name = person_display_name(person);
                    for email in &dupes {
                        println!("{}{} | {}", prefix, name, email);
                    }
                }
                count += dupes.len();

                if fix && !dry_run && !quiet {
                    let name = person_display_name(person);
                    if prompt_yes_no(&format!("Remove duplicate \"{}\" from {}?", dupes.join(", "), name))? {
                        remove_duplicate_emails(hub, person).await?;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

async fn remove_duplicate_emails(hub: &HubType, person: &google_people1::api::Person) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut emails) = updated.email_addresses {
        let mut seen = std::collections::HashSet::new();
        emails.retain(|e| {
            let val = e.value.as_deref().unwrap_or("");
            seen.insert(val.to_string())
        });
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
        .doit()
        .await?;
    eprintln!("  Removed duplicates for {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

pub async fn cmd_check_contact_email_duplicate(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses"]).await?;
    check_duplicate_emails(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_phone_label_missing(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            let has_untyped = nums.iter().any(|pn| !phone_has_type(pn));
            if has_untyped {
                if !quiet {
                    if count == 0 {
                        if let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    }
                    let name = person_display_name(person);
                    for pn in nums.iter().filter(|pn| !phone_has_type(pn)) {
                        let phone = pn.value.as_deref().unwrap_or("");
                        println!("{}{} | {}", prefix, name, phone);
                    }
                }
                let untyped_count = nums.iter().filter(|pn| !phone_has_type(pn)).count();
                count += untyped_count;

                if fix && !dry_run && !quiet {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let name = person_display_name(person);
                    if let Some(new_label) = prompt_phone_label_fix(&name)? {
                        let resource_name = person
                            .resource_name
                            .as_deref()
                            .ok_or("Contact missing resource name")?;
                        let mut updated = person.clone();
                        if let Some(ref mut phone_nums) = updated.phone_numbers {
                            for pn in phone_nums.iter_mut() {
                                if !phone_has_type(pn) {
                                    pn.type_ = Some(new_label.clone());
                                    pn.formatted_type = Some(new_label.clone());
                                }
                            }
                        }
                        hub.people()
                            .update_contact(updated, resource_name)
                            .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                            .doit()
                            .await?;
                        eprintln!("  Fixed labels for {}", name);
                        tokio::time::sleep(MUTATE_DELAY).await;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_phone_label_missing(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers"]).await?;
    check_phone_label_missing(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_phone_label_english(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            let has_non_english = nums.iter().any(|pn| {
                let label = get_phone_label(pn);
                !label.is_empty() && !label.chars().all(|c| c.is_ascii())
            });
            if has_non_english {
                if !quiet {
                    if count == 0 {
                        if let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    }
                    let name = person_display_name(person);
                    for pn in nums {
                        let label = get_phone_label(pn);
                        if !label.is_empty() && !label.chars().all(|c| c.is_ascii()) {
                            let phone = pn.value.as_deref().unwrap_or("");
                            println!("{}{} | {} [{}]", prefix, name, phone, label);
                        }
                    }
                }
                count += nums.iter().filter(|pn| {
                    let label = get_phone_label(pn);
                    !label.is_empty() && !label.chars().all(|c| c.is_ascii())
                }).count();

                if fix && !dry_run && !quiet {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    let name = person_display_name(person);
                    let new_label = prompt_phone_label_fix(&name)?;
                    if let Some(new_label) = new_label {
                        fix_phone_labels_english(hub, person, &new_label).await?;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_phone_label_english(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers"]).await?;
    check_phone_label_english(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn fix_phone_labels_english(hub: &HubType, person: &google_people1::api::Person, new_label: &str) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    let mut updated = person.clone();
    if let Some(ref mut nums) = updated.phone_numbers {
        for pn in nums.iter_mut() {
            let label = get_phone_label(pn);
            if !label.is_empty() && !label.chars().all(|c| c.is_ascii()) {
                pn.type_ = Some(new_label.to_string());
                pn.formatted_type = Some(new_label.to_string());
            }
        }
    }
    hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
        .doit()
        .await?;
    eprintln!("  Fixed labels for {}", person_display_name(person));
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

pub async fn cmd_check_contact_no_label(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_fields = &[
        "names", "emailAddresses", "phoneNumbers", "addresses", "birthdays",
        "organizations", "memberships", "biographies", "urls", "events",
        "relations", "nicknames", "occupations", "interests", "skills",
        "userDefined", "imClients", "sipAddresses", "locations",
        "externalIds", "clientData",
    ];
    let contacts = if fix {
        fetch_all_contacts(&hub, all_fields).await?
    } else {
        fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "memberships"]).await?
    };

    let (user_groups_owned, label_names) = if fix {
        let all_groups = fetch_all_contact_groups(&hub).await?;
        let ug: Vec<(String, String)> = all_groups.iter()
            .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
            .filter_map(|g| {
                let name = g.name.as_deref()?;
                let rn = g.resource_name.as_deref()?;
                Some((name.to_string(), rn.to_string()))
            })
            .collect();
        let ln: Vec<String> = ug.iter().map(|(name, _)| name.clone()).collect();
        (ug, ln)
    } else {
        (vec![], vec![])
    };
    let user_groups: Vec<(&str, &str)> = user_groups_owned.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();

    check_no_label(&hub, &contacts, fix, dry_run, "", None, false, &user_groups, &label_names).await?;
    Ok(())
}

async fn prompt_label_autocomplete(
    hub: &HubType,
    label_names: &[String],
    user_groups: &[(&str, &str)],
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    use rustyline::completion::{Completer, Pair};
    use rustyline::Editor;
    use rustyline::highlight::Highlighter;
    use rustyline::hint::Hinter;
    use rustyline::validate::Validator;
    use rustyline::Helper;

    struct LabelCompleter {
        labels: Vec<String>,
    }

    impl Helper for LabelCompleter {}
    impl Highlighter for LabelCompleter {}
    impl Hinter for LabelCompleter {
        type Hint = String;
    }
    impl Validator for LabelCompleter {}

    impl Completer for LabelCompleter {
        type Candidate = Pair;
        fn complete(
            &self,
            line: &str,
            pos: usize,
            _ctx: &rustyline::Context<'_>,
        ) -> rustyline::Result<(usize, Vec<Pair>)> {
            let input = &line[..pos].to_lowercase();
            let matches: Vec<Pair> = self.labels.iter()
                .filter(|l| l.to_lowercase().starts_with(input))
                .map(|l| Pair { display: l.clone(), replacement: l.clone() })
                .collect();
            Ok((0, matches))
        }
    }

    let completer = LabelCompleter { labels: label_names.to_vec() };
    let mut rl = Editor::new()?;
    rl.set_helper(Some(completer));
    match rl.readline("  Label (tab to complete, or type new name): ") {
        Ok(line) => {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            // Find matching group (case-insensitive)
            let lower = trimmed.to_lowercase();
            if let Some((_, rn)) = user_groups.iter().find(|(name, _)| name.to_lowercase() == lower) {
                Ok(Some(rn.to_string()))
            } else {
                // Create a new label
                if prompt_yes_no(&format!("Label \"{}\" does not exist. Create it?", trimmed))? {
                    let mut new_group = google_people1::api::ContactGroup::default();
                    new_group.name = Some(trimmed.to_string());
                    let req = google_people1::api::CreateContactGroupRequest {
                        contact_group: Some(new_group),
                        read_group_fields: None,
                    };
                    let (_, created) = hub.contact_groups().create(req).doit().await?;
                    let rn = created.resource_name
                        .ok_or("Created group missing resource name")?;
                    eprintln!("  Created label \"{}\"", trimmed);
                    tokio::time::sleep(MUTATE_DELAY).await;
                    Ok(Some(rn))
                } else {
                    Ok(None)
                }
            }
        }
        Err(_) => Ok(None),
    }
}

pub async fn cmd_check_contact_label_nophone(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;

    let empty_groups: Vec<&google_people1::api::ContactGroup> = all_groups.iter().filter(|g| {
        let count = g.member_count.unwrap_or(0);
        let is_user_group = g.group_type.as_deref() == Some("USER_CONTACT_GROUP");
        count == 0 && is_user_group
    }).collect();

    for group in &empty_groups {
        let name = group.name.as_deref().unwrap_or("<unnamed>");
        println!("{}", name);

        if fix && !dry_run {
            use std::io::Write;
            std::io::stdout().flush()?;
            if let Some(resource_name) = group.resource_name.as_deref() {
                if prompt_yes_no(&format!("Delete label \"{}\"?", name))? {
                    hub.contact_groups().delete(resource_name).doit().await?;
                    eprintln!("  Deleted.");
                    tokio::time::sleep(MUTATE_DELAY).await;
                } else {
                    eprintln!("  Skipped.");
                }
            }
        }
    }

    Ok(())
}

pub async fn cmd_show_contact(search: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_fields = &[
        "names", "emailAddresses", "phoneNumbers", "addresses", "birthdays",
        "organizations", "memberships", "biographies", "urls", "events",
        "relations", "nicknames", "occupations", "interests", "skills",
        "userDefined", "imClients", "sipAddresses", "locations",
        "externalIds", "clientData", "metadata",
    ];
    let contacts = fetch_all_contacts(&hub, all_fields).await?;
    let search_lower = search.to_lowercase();
    let matches: Vec<_> = contacts.iter().filter(|p| {
        person_name(p).to_lowercase().contains(&search_lower)
    }).collect();

    if matches.is_empty() {
        println!("No contacts found matching \"{}\"", search);
        return Ok(());
    }

    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names: std::collections::HashMap<String, String> = all_groups.iter()
        .filter_map(|g| {
            let rn = g.resource_name.as_deref()?;
            let name = g.name.as_deref()?;
            Some((rn.to_string(), name.to_string()))
        })
        .collect();

    for (i, person) in matches.iter().enumerate() {
        if i > 0 { println!("\n{}", "=".repeat(60)); }
        print_person_details(person, Some(&group_names));
    }
    Ok(())
}

fn print_person_details(person: &google_people1::api::Person, group_names: Option<&std::collections::HashMap<String, String>>) {
    let name = person_display_name(person);
    println!("Name: {}", name);

    if let Some(names) = &person.names {
        for n in names {
            if let Some(given) = &n.given_name { println!("  Given name: {}", given); }
            if let Some(family) = &n.family_name { println!("  Family name: {}", family); }
            if let Some(middle) = &n.middle_name { println!("  Middle name: {}", middle); }
            if let Some(prefix) = &n.honorific_prefix { println!("  Prefix: {}", prefix); }
            if let Some(suffix) = &n.honorific_suffix { println!("  Suffix: {}", suffix); }
        }
    }

    if let Some(nicknames) = &person.nicknames {
        for n in nicknames {
            if let Some(val) = &n.value { println!("Nickname: {}", val); }
        }
    }

    if let Some(emails) = &person.email_addresses {
        for e in emails {
            let val = e.value.as_deref().unwrap_or("");
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("Email: {}", val); }
            else { println!("Email: {} [{}]", val, t); }
        }
    }

    if let Some(phones) = &person.phone_numbers {
        for p in phones {
            let val = p.value.as_deref().unwrap_or("");
            let t = p.formatted_type.as_deref().or(p.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("Phone: {}", val); }
            else { println!("Phone: {} [{}]", val, t); }
        }
    }

    if let Some(addrs) = &person.addresses {
        for a in addrs {
            let t = a.formatted_type.as_deref().or(a.type_.as_deref()).unwrap_or("");
            if let Some(formatted) = &a.formatted_value {
                if t.is_empty() { println!("Address: {}", formatted); }
                else { println!("Address [{}]: {}", t, formatted); }
            }
        }
    }

    if let Some(orgs) = &person.organizations {
        for o in orgs {
            let org_name = o.name.as_deref().unwrap_or("");
            let title = o.title.as_deref().unwrap_or("");
            let dept = o.department.as_deref().unwrap_or("");
            let mut parts = Vec::new();
            if !title.is_empty() { parts.push(title.to_string()); }
            if !org_name.is_empty() { parts.push(org_name.to_string()); }
            if !dept.is_empty() { parts.push(format!("({})", dept)); }
            if !parts.is_empty() { println!("Organization: {}", parts.join(", ")); }
        }
    }

    if let Some(bdays) = &person.birthdays {
        for b in bdays {
            if let Some(date) = &b.date {
                let y = date.year.unwrap_or(0);
                let m = date.month.unwrap_or(0);
                let d = date.day.unwrap_or(0);
                if y > 0 { println!("Birthday: {}-{:02}-{:02}", y, m, d); }
                else { println!("Birthday: {:02}-{:02}", m, d); }
            }
            if let Some(text) = &b.text { println!("Birthday: {}", text); }
        }
    }

    if let Some(relations) = &person.relations {
        for r in relations {
            let val = r.person.as_deref().unwrap_or("");
            let t = r.formatted_type.as_deref().or(r.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("Relation: {}", val); }
            else { println!("Relation: {} [{}]", val, t); }
        }
    }

    if let Some(events) = &person.events {
        for e in events {
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("event");
            if let Some(date) = &e.date {
                let y = date.year.unwrap_or(0);
                let m = date.month.unwrap_or(0);
                let d = date.day.unwrap_or(0);
                if y > 0 { println!("Event [{}]: {}-{:02}-{:02}", t, y, m, d); }
                else { println!("Event [{}]: {:02}-{:02}", t, m, d); }
            }
        }
    }

    if let Some(bios) = &person.biographies {
        for b in bios {
            if let Some(val) = &b.value { println!("Biography: {}", val); }
        }
    }

    if let Some(urls) = &person.urls {
        for u in urls {
            let val = u.value.as_deref().unwrap_or("");
            let t = u.formatted_type.as_deref().or(u.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("URL: {}", val); }
            else { println!("URL: {} [{}]", val, t); }
        }
    }

    if let Some(ims) = &person.im_clients {
        for im in ims {
            let val = im.username.as_deref().unwrap_or("");
            let proto = im.formatted_protocol.as_deref().or(im.protocol.as_deref()).unwrap_or("");
            if proto.is_empty() { println!("IM: {}", val); }
            else { println!("IM: {} [{}]", val, proto); }
        }
    }

    if let Some(sips) = &person.sip_addresses {
        for s in sips {
            if let Some(val) = &s.value { println!("SIP: {}", val); }
        }
    }

    if let Some(occupations) = &person.occupations {
        for o in occupations {
            if let Some(val) = &o.value { println!("Occupation: {}", val); }
        }
    }

    if let Some(interests) = &person.interests {
        for i in interests {
            if let Some(val) = &i.value { println!("Interest: {}", val); }
        }
    }

    if let Some(skills) = &person.skills {
        for s in skills {
            if let Some(val) = &s.value { println!("Skill: {}", val); }
        }
    }

    if let Some(locations) = &person.locations {
        for l in locations {
            if let Some(val) = &l.value { println!("Location: {}", val); }
        }
    }

    if let Some(ext_ids) = &person.external_ids {
        for e in ext_ids {
            let val = e.value.as_deref().unwrap_or("");
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
            if t.is_empty() { println!("External ID: {}", val); }
            else { println!("External ID: {} [{}]", val, t); }
        }
    }

    if let Some(user_defined) = &person.user_defined {
        for u in user_defined {
            let key = u.key.as_deref().unwrap_or("");
            let val = u.value.as_deref().unwrap_or("");
            println!("Custom: {} = {}", key, val);
        }
    }

    if let Some(client_data) = &person.client_data {
        for c in client_data {
            let key = c.key.as_deref().unwrap_or("");
            let val = c.value.as_deref().unwrap_or("");
            println!("Client data: {} = {}", key, val);
        }
    }

    if let Some(memberships) = &person.memberships {
        let labels: Vec<String> = memberships.iter().filter_map(|m| {
            let rn = m.contact_group_membership.as_ref()?.contact_group_resource_name.as_deref()?;
            if rn == "contactGroups/myContacts" { return None; }
            let display = group_names
                .and_then(|gn| gn.get(rn))
                .map(|s| s.as_str())
                .unwrap_or(rn);
            Some(display.to_string())
        }).collect();
        if !labels.is_empty() {
            println!("Labels: {}", labels.join(", "));
        }
    }

    if let Some(rn) = &person.resource_name {
        println!("Resource: {}", rn);
    }
}

pub async fn cmd_check_contact_label_space(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;

    let with_space: Vec<&google_people1::api::ContactGroup> = all_groups.iter().filter(|g| {
        g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
            && g.name.as_deref().unwrap_or("").contains(' ')
    }).collect();

    for group in &with_space {
        let name = group.name.as_deref().unwrap_or("<unnamed>");
        println!("{}", name);

        if fix && !dry_run {
            use std::io::Write;
            std::io::stdout().flush()?;
            if let Some(new_name) = prompt_rename_label(name)? {
                let resource_name = group.resource_name.as_deref()
                    .ok_or("Contact group missing resource name")?;
                let mut updated_group = (*group).clone();
                updated_group.name = Some(new_name.clone());
                let req = google_people1::api::UpdateContactGroupRequest {
                    contact_group: Some(updated_group),
                    read_group_fields: None,
                    update_group_fields: None,
                };
                hub.contact_groups().update(req, resource_name).doit().await?;
                eprintln!("  Renamed \"{}\" -> \"{}\"", name, new_name);
                tokio::time::sleep(MUTATE_DELAY).await;
            } else {
                eprintln!("  Skipped.");
            }
        }
    }

    Ok(())
}

pub async fn cmd_check_contact_label_camelcase(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;

    let not_camelcase: Vec<&google_people1::api::ContactGroup> = all_groups.iter().filter(|g| {
        g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
            && g.name.as_deref().is_some_and(|n| n.starts_with(char::is_lowercase))
    }).collect();

    for group in &not_camelcase {
        let name = group.name.as_deref().unwrap_or("<unnamed>");
        let camel = capitalize_first(name);
        if fix || dry_run {
            println!("{} -> {}", name, camel);
        } else {
            println!("{}", name);
        }

        if fix && !dry_run {
            let resource_name = group.resource_name.as_deref()
                .ok_or("Contact group missing resource name")?;
            let mut updated_group = (*group).clone();
            updated_group.name = Some(camel.clone());
            let req = google_people1::api::UpdateContactGroupRequest {
                contact_group: Some(updated_group),
                read_group_fields: None,
                update_group_fields: None,
            };
            hub.contact_groups().update(req, resource_name).doit().await?;
            eprintln!("  Renamed \"{}\" -> \"{}\"", name, camel);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
    }

    Ok(())
}

pub async fn cmd_remove_label_from_all_contacts(label: &str, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;

    // Find the group by name (case-insensitive)
    let lower = label.to_lowercase();
    let group = all_groups.iter().find(|g| {
        g.name.as_deref().is_some_and(|n| n.to_lowercase() == lower)
    });
    let group = match group {
        Some(g) => g,
        None => {
            eprintln!("Label \"{}\" not found.", label);
            return Ok(());
        }
    };
    let group_rn = group.resource_name.as_deref()
        .ok_or("Contact group missing resource name")?;
    let group_name = group.name.as_deref().unwrap_or(label);

    // Find all contacts that have this label
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "memberships"]).await?;
    let members: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        p.memberships.as_ref().is_some_and(|ms| {
            ms.iter().any(|m| {
                m.contact_group_membership.as_ref().is_some_and(|cgm| {
                    cgm.contact_group_resource_name.as_deref() == Some(group_rn)
                })
            })
        })
    }).collect();

    if members.is_empty() {
        println!("No contacts have the label \"{}\".", group_name);
        return Ok(());
    }

    println!("Removing label \"{}\" from {} contacts:", group_name, members.len());
    for person in &members {
        let name = person_display_name(person);
        println!("  {}", name);
    }

    if !dry_run {
        // Remove in batches — the API accepts up to 1000 resource names per call
        let resource_names: Vec<String> = members.iter()
            .filter_map(|p| p.resource_name.as_ref().cloned())
            .collect();
        for chunk in resource_names.chunks(1000) {
            let req = google_people1::api::ModifyContactGroupMembersRequest {
                resource_names_to_add: None,
                resource_names_to_remove: Some(chunk.to_vec()),
            };
            hub.contact_groups().members_modify(req, group_rn).doit().await?;
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        eprintln!("Done. Removed label \"{}\" from {} contacts.", group_name, members.len());
    }

    Ok(())
}

pub async fn cmd_show_phone_labels() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["phoneNumbers"]).await?;
    let mut labels = std::collections::BTreeSet::new();
    for person in &contacts {
        if let Some(nums) = &person.phone_numbers {
            for pn in nums {
                let label = get_phone_label(pn);
                if !label.is_empty() {
                    labels.insert(label.to_string());
                }
            }
        }
    }
    for label in &labels {
        println!("{}", label);
    }
    Ok(())
}

pub async fn cmd_review_phone_label(label: &str, fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers"]).await?;
    let label_lower = label.to_lowercase();
    let mut count = 0;
    for person in &contacts {
        if let Some(nums) = &person.phone_numbers {
            let matching: Vec<_> = nums.iter().enumerate()
                .filter(|(_, pn)| get_phone_label(pn).to_lowercase() == label_lower)
                .collect();
            if matching.is_empty() { continue; }
            let name = person_display_name(person);
            for (idx, pn) in &matching {
                let phone = pn.value.as_deref().unwrap_or("");
                let pn_label = get_phone_label(pn);
                println!("{} | {} [{}]", name, phone, pn_label);
                count += 1;
                if fix {
                    if dry_run {
                        eprintln!("  (dry-run) would prompt for action");
                        continue;
                    }
                    use std::io::Write;
                    loop {
                        eprint!("  [d]elete / [r]elabel / [s]kip: ");
                        std::io::stderr().flush()?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        match input.trim().chars().next() {
                            Some('d') => {
                                let resource_name = person.resource_name.as_deref()
                                    .ok_or("Contact missing resource name")?;
                                let mut updated = person.clone();
                                if let Some(ref mut phone_nums) = updated.phone_numbers {
                                    phone_nums.remove(*idx);
                                }
                                hub.people()
                                    .update_contact(updated, resource_name)
                                    .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                                    .doit()
                                    .await?;
                                eprintln!("  Deleted phone {} from {}", phone, name);
                                tokio::time::sleep(MUTATE_DELAY).await;
                                break;
                            }
                            Some('r') => {
                                if let Some(new_label) = prompt_phone_label_fix(&name)? {
                                    let resource_name = person.resource_name.as_deref()
                                        .ok_or("Contact missing resource name")?;
                                    let mut updated = person.clone();
                                    if let Some(ref mut phone_nums) = updated.phone_numbers {
                                        phone_nums[*idx].type_ = Some(new_label.clone());
                                        phone_nums[*idx].formatted_type = Some(new_label);
                                    }
                                    hub.people()
                                        .update_contact(updated, resource_name)
                                        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                                        .doit()
                                        .await?;
                                    eprintln!("  Relabeled phone for {}", name);
                                    tokio::time::sleep(MUTATE_DELAY).await;
                                }
                                break;
                            }
                            Some('s') => {
                                eprintln!("  Skipped.");
                                break;
                            }
                            _ => eprintln!("  Invalid choice. Enter d, r, or s."),
                        }
                    }
                }
            }
        }
    }
    if count == 0 {
        println!("No phones found with label \"{}\"", label);
    } else {
        println!("{} phone(s) with label \"{}\"", count, label);
    }
    Ok(())
}

pub async fn cmd_show_contact_labels() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    for group in &all_groups {
        let name = group.name.as_deref().unwrap_or("<unnamed>");
        let count = group.member_count.unwrap_or(0);
        let kind = match group.group_type.as_deref() {
            Some("USER_CONTACT_GROUP") => "user",
            Some("SYSTEM_CONTACT_GROUP") => "system",
            Some(other) => other,
            None => "unknown",
        };
        println!("{} ({}) [{}]", name, count, kind);
    }
    Ok(())
}

pub async fn cmd_check_all(fix: bool, dry_run: bool, stats: bool, verbose: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let skip: std::collections::HashSet<&str> = config.check_all.skip.iter().map(|s| s.as_str()).collect();

    let hub = build_hub().await?;
    let all_contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "phoneNumbers", "memberships", "organizations"]).await?;
    let all_groups_for_regexp = fetch_all_contact_groups(&hub).await?;
    let group_names_for_regexp = build_group_name_map(&all_groups_for_regexp);

    let (user_groups_owned_regexp, label_names_regexp) = if fix {
        let ug: Vec<(String, String)> = all_groups_for_regexp.iter()
            .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
            .filter_map(|g| {
                let name = g.name.as_deref()?;
                let rn = g.resource_name.as_deref()?;
                Some((name.to_string(), rn.to_string()))
            })
            .collect();
        let ln: Vec<String> = ug.iter().map(|(name, _)| name.clone()).collect();
        (ug, ln)
    } else {
        (vec![], vec![])
    };
    let user_groups_regexp: Vec<(&str, &str)> = user_groups_owned_regexp.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();

    let mut results: Vec<(&str, usize)> = Vec::new();

    let prefix = if stats { "" } else { "  " };
    let hdr = |s: &'static str| -> Option<&'static str> { if stats { None } else { Some(s) } };
    let log = |name: &str| { if verbose { eprintln!("Running {}...", name); } };

    if !skip.contains("check-phone-countrycode") {
        log("check-phone-countrycode");
        let country_owned = country.to_string();
        let no_country = check_phone_issues(
            &hub, &all_contacts,
            |v| is_fixable_phone(v) && !has_country_code(v),
            move |v| add_country_code(v, &country_owned),
            fix, dry_run, prefix, hdr("Phones missing country code (check-phone-countrycode)"), stats,
        ).await?;
        results.push(("check-phone-countrycode", no_country));
    }

    if !skip.contains("check-phone-format") {
        log("check-phone-format");
        let country_owned2 = country.to_string();
        let bad_format = check_phone_issues(
            &hub, &all_contacts,
            |v| is_fixable_phone(v) && !is_correct_phone_format(v),
            move |v| fix_phone_format(v, &country_owned2),
            fix, dry_run, prefix, hdr("Phones not in +CC-NUMBER format (check-phone-format)"), stats,
        ).await?;
        results.push(("check-phone-format", bad_format));
    }

    if !skip.contains("check-contact-given-name-regexp") && config.check_contact_given_name_regexp.allow.is_some() {
        log("check-contact-given-name-regexp");
        let given_name_regexp = check_given_name_regexp(&hub, &all_contacts, &group_names_for_regexp, &config.check_contact_given_name_regexp, fix, dry_run, prefix, hdr("Given name doesn't match allow regex (check-contact-given-name-regexp)"), stats, &user_groups_regexp, &label_names_regexp).await?;
        results.push(("check-contact-given-name-regexp", given_name_regexp));
    }

    if !skip.contains("check-contact-family-name-regexp") && config.check_contact_family_name_regexp.allow.is_some() {
        log("check-contact-family-name-regexp");
        let family_name_regexp = check_family_name_regexp(&hub, &all_contacts, &group_names_for_regexp, &config.check_contact_family_name_regexp, fix, dry_run, prefix, hdr("Family name doesn't match allow regex (check-contact-family-name-regexp)"), stats, &user_groups_regexp, &label_names_regexp).await?;
        results.push(("check-contact-family-name-regexp", family_name_regexp));
    }

    if !skip.contains("check-contact-suffix-regexp") {
        log("check-contact-suffix-regexp");
        let suffix_regexp = check_suffix_regexp(&hub, &all_contacts, &group_names_for_regexp, &config.check_contact_suffix_regexp, fix, dry_run, prefix, hdr("Suffix doesn't match allow regex (check-contact-suffix-regexp)"), stats).await?;
        results.push(("check-contact-suffix-regexp", suffix_regexp));
    }

    if !skip.contains("check-contact-displayname-duplicate") {
        log("check-contact-displayname-duplicate");
        let name_dup = check_name_duplicate(&hub, &all_contacts, fix, dry_run, prefix, hdr("Duplicate contact names (check-contact-displayname-duplicate)"), stats).await?;
        results.push(("check-contact-displayname-duplicate", name_dup));
    }

    // For check-contact-no-label with fix, we need contact groups for label autocomplete
    let (user_groups_owned, label_names) = if fix {
        let all_groups_for_labels = fetch_all_contact_groups(&hub).await?;
        let ug: Vec<(String, String)> = all_groups_for_labels.iter()
            .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
            .filter_map(|g| {
                let name = g.name.as_deref()?;
                let rn = g.resource_name.as_deref()?;
                Some((name.to_string(), rn.to_string()))
            })
            .collect();
        let ln: Vec<String> = ug.iter().map(|(name, _)| name.clone()).collect();
        (ug, ln)
    } else {
        (vec![], vec![])
    };
    let user_groups: Vec<(&str, &str)> = user_groups_owned.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();

    if !skip.contains("check-contact-no-label") {
        log("check-contact-no-label");
        let no_label = check_no_label(&hub, &all_contacts, fix, dry_run, prefix, hdr("Contacts without label (check-contact-no-label)"), stats, &user_groups, &label_names).await?;
        results.push(("check-contact-no-label", no_label));
    }

    if !skip.contains("check-phone-label-missing") {
        log("check-phone-label-missing");
        let phone_no_label = check_phone_label_missing(&hub, &all_contacts, fix, dry_run, prefix, hdr("Phones without label (check-phone-label-missing)"), stats).await?;
        results.push(("check-phone-label-missing", phone_no_label));
    }

    if !skip.contains("check-phone-label-english") {
        log("check-phone-label-english");
        let phone_label_eng = check_phone_label_english(&hub, &all_contacts, fix, dry_run, prefix, hdr("Non-English phone labels (check-phone-label-english)"), stats).await?;
        results.push(("check-phone-label-english", phone_label_eng));
    }

    if !skip.contains("check-contact-email") {
        log("check-contact-email");
        let email_issues = check_email(&hub, &all_contacts, fix, dry_run, prefix, hdr("Invalid or uppercase emails (check-contact-email)"), stats).await?;
        results.push(("check-contact-email", email_issues));
    }

    if !skip.contains("check-phone-duplicate") {
        log("check-phone-duplicate");
        let dup_phones = check_duplicate_phones(&hub, &all_contacts, fix, dry_run, prefix, hdr("Duplicate phone numbers (check-phone-duplicate)"), stats).await?;
        results.push(("check-phone-duplicate", dup_phones));
    }

    if !skip.contains("check-contact-email-duplicate") {
        log("check-contact-email-duplicate");
        let dup_emails = check_duplicate_emails(&hub, &all_contacts, fix, dry_run, prefix, hdr("Duplicate email addresses (check-contact-email-duplicate)"), stats).await?;
        results.push(("check-contact-email-duplicate", dup_emails));
    }

    // Check for empty labels (contact groups) — separate API call
    let all_groups = fetch_all_contact_groups(&hub).await?;

    if !skip.contains("check-contact-label-nophone") {
        log("check-contact-label-nophone");
        let empty: Vec<_> = all_groups.iter().filter(|g| {
            g.member_count.unwrap_or(0) == 0
                && g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
        }).collect();
        if !stats && !empty.is_empty() {
            println!("=== Empty labels (check-contact-label-nophone) ({}) ===", empty.len());
            for group in &empty {
                let name = group.name.as_deref().unwrap_or("<unnamed>");
                println!("  {}", name);

                if fix && !dry_run {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    if let Some(resource_name) = group.resource_name.as_deref() {
                        if prompt_yes_no(&format!("Delete label \"{}\"?", name))? {
                            hub.contact_groups().delete(resource_name).doit().await?;
                            eprintln!("  Deleted.");
                            tokio::time::sleep(MUTATE_DELAY).await;
                        } else {
                            eprintln!("  Skipped.");
                        }
                    }
                }
            }
            println!();
        }
        results.push(("check-contact-label-nophone", empty.len()));
    }

    if !skip.contains("check-contact-label-space") {
        log("check-contact-label-space");
        let with_space: Vec<_> = all_groups.iter().filter(|g| {
            g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
                && g.name.as_deref().unwrap_or("").contains(' ')
        }).collect();
        if !stats && !with_space.is_empty() {
            println!("=== Labels with spaces (check-contact-label-space) ({}) ===", with_space.len());
            for group in &with_space {
                let name = group.name.as_deref().unwrap_or("<unnamed>");
                println!("  {}", name);

                if fix && !dry_run {
                    use std::io::Write;
                    std::io::stdout().flush()?;
                    if let Some(new_name) = prompt_rename_label(name)? {
                        let resource_name = group.resource_name.as_deref()
                            .ok_or("Contact group missing resource name")?;
                        let mut updated_group = (*group).clone();
                        updated_group.name = Some(new_name.clone());
                        let req = google_people1::api::UpdateContactGroupRequest {
                            contact_group: Some(updated_group),
                            read_group_fields: None,
                            update_group_fields: None,
                        };
                        hub.contact_groups().update(req, resource_name).doit().await?;
                        eprintln!("  Renamed \"{}\" -> \"{}\"", name, new_name);
                        tokio::time::sleep(MUTATE_DELAY).await;
                    } else {
                        eprintln!("  Skipped.");
                    }
                }
            }
            println!();
        }
        results.push(("check-contact-label-space", with_space.len()));
    }

    if !skip.contains("check-contact-label-camelcase") {
        log("check-contact-label-camelcase");
        let not_camelcase: Vec<_> = all_groups.iter().filter(|g| {
            g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
                && g.name.as_deref().is_some_and(|n| n.starts_with(char::is_lowercase))
        }).collect();
        if !stats && !not_camelcase.is_empty() {
            println!("=== Labels not camelCase (check-contact-label-camelcase) ({}) ===", not_camelcase.len());
            for group in &not_camelcase {
                let name = group.name.as_deref().unwrap_or("<unnamed>");
                let camel = capitalize_first(name);
                if fix || dry_run {
                    println!("  {} -> {}", name, camel);
                } else {
                    println!("  {}", name);
                }

                if fix && !dry_run {
                    let resource_name = group.resource_name.as_deref()
                        .ok_or("Contact group missing resource name")?;
                    let mut updated_group = (*group).clone();
                    updated_group.name = Some(camel.clone());
                    let req = google_people1::api::UpdateContactGroupRequest {
                        contact_group: Some(updated_group),
                        read_group_fields: None,
                        update_group_fields: None,
                    };
                    hub.contact_groups().update(req, resource_name).doit().await?;
                    eprintln!("  Renamed \"{}\" -> \"{}\"", name, camel);
                    tokio::time::sleep(MUTATE_DELAY).await;
                }
            }
            println!();
        }
        results.push(("check-contact-label-camelcase", not_camelcase.len()));
    }

    if stats {
        let total: usize = results.iter().map(|(_, c)| c).sum();
        for (name, count) in &results {
            if *count > 0 {
                println!("{}: {}", name, count);
            }
        }
        if total == 0 {
            println!("All checks passed!");
        } else {
            println!("---");
            println!("Total: {}", total);
        }
    } else {
        let found_any = results.iter().any(|(_, c)| *c > 0);
        if !found_any {
            println!("All checks passed!");
        }
    }

    Ok(())
}
