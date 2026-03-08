use google_people1::FieldMask;

use crate::helpers::*;

struct CheckContext<'a> {
    fix: bool,
    dry_run: bool,
    prefix: &'a str,
    header: Option<&'a str>,
    quiet: bool,
    user_groups: &'a [(&'a str, &'a str)],
    label_names: &'a [String],
    group_names: &'a std::collections::HashMap<String, String>,
}

const DEFAULT_CONFIG: &str = r#"# rscontacts configuration

[check-all]
# List of checks to skip when running check-all.
# All checks not listed here will run by default.
# Uncomment any check name to skip it.
skip = [
    # "check-contact-given-name-regexp",
    # "check-contact-family-name-regexp",
    # "check-contact-suffix-regexp",
    # "check-contact-no-given-name",
    # "check-contact-no-identity",
    # "check-contact-name-is-company",
    # "check-contact-company-known",
    # "check-contact-displayname-duplicate",
    # "check-contact-type",
    # "check-contact-no-middle-name",
    # "check-contact-no-nickname",
    # "check-contact-no-label",
    # "check-contact-email",
    # "check-contact-email-duplicate",
    # "check-contact-label-nophone",
    # "check-contact-label-regexp",
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

# Allow regex for contact labels (groups). Labels that do NOT match
# this pattern will be flagged by check-contact-label-regexp.
# CamelCase: starts with uppercase, then any mix of upper/lowercase letters.
[check-contact-label-regexp]
allow = '^[A-Z][a-zA-Z]*$'

# List of company names. Contacts whose given or family name matches
# a company name (case-insensitive) will be flagged by check-contact-name-is-company.
# [check-contact-name-is-company]
# companies = ["Google", "Microsoft", "Apple"]

# List of allowed given names (case-sensitive).
# Contacts whose given name is NOT in this list will be flagged
# by check-contact-given-name-known.
# [check-contact-given-name-known]
# names = ["John", "Jane", "Mark"]
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
    let _ = emails; // emails are now always shown via format_person_line
    let hub = build_hub().await?;
    let fields = vec!["names", "organizations", "phoneNumbers", "nicknames", "emailAddresses", "memberships"];
    let contacts = fetch_all_contacts(&hub, &fields).await?;

    let contacts: Vec<_> = if starred {
        contacts.into_iter().filter(is_starred).collect()
    } else {
        contacts
    };

    let group_names = if labels {
        let all_groups = fetch_all_contact_groups(&hub).await?;
        build_group_name_map(&all_groups)
    } else {
        std::collections::HashMap::new()
    };

    let gn = if labels { Some(&group_names) } else { None };

    for person in &contacts {
        println!("{}", format_person_line(person, gn));
    }

    Ok(())
}

async fn check_phone_issues<P, T>(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    predicate: P,
    transform: T,
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>>
where
    P: Fn(&str) -> bool,
    T: Fn(&str) -> String + Clone,
{
    let filtered: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        p.phone_numbers.as_ref().is_some_and(|nums| nums.iter().any(|pn| {
            pn.value.as_deref().is_some_and(&predicate)
        }))
    }).collect();

    if !ctx.quiet {
        if !filtered.is_empty()
            && let Some(header) = ctx.header {
                println!("=== {} ({}) ===", header, filtered.len());
            }

        for person in &filtered {
            println!("{}{}", ctx.prefix, format_person_line(person, None));

            if let Some(nums) = &person.phone_numbers {
                for pn in nums {
                    if let Some(val) = pn.value.as_deref()
                        && predicate(val) {
                            let fixed = transform(val);
                            if ctx.fix || ctx.dry_run {
                                println!("{}  {} -> {}", ctx.prefix, val, fixed);
                            }
                        }
                }
            }

            if ctx.fix && !ctx.dry_run {
                if prompt_yes_no("  Fix?")? {
                    let transform = transform.clone();
                    update_phone_numbers(hub, person, |val| {
                        if predicate(val) {
                            Some(transform(val))
                        } else {
                            None
                        }
                    }).await?;
                } else {
                    eprintln!("  Skipped.");
                }
            }
        }

        if !filtered.is_empty() && ctx.header.is_some() {
            println!();
        }
    }

    Ok(filtered.len())
}

pub async fn cmd_check_contact_given_name_regexp(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_given_name_regexp(&hub, &contacts, &config.check_contact_given_name_regexp, &ctx).await?;
    Ok(())
}

async fn check_given_name_regexp(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    config: &crate::helpers::NameRegexpConfig,
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = match &config.allow {
        Some(p) => p,
        None => {
            if !ctx.quiet {
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
            if !ctx.quiet {
                if count == 0
                    && let Some(header) = ctx.header {
                        println!("=== {} ===", header);
                    }
                println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

                if ctx.fix && !ctx.dry_run {
                    interactive_given_name_fix(hub, person, given, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                }
            }
            count += 1;
        }
    }
    if !ctx.quiet && count > 0 && ctx.header.is_some() { println!(); }
    Ok(count)
}

async fn interactive_given_name_fix(
    hub: &HubType,
    person: &google_people1::api::Person,
    given: &str,
    user_groups: &[(&str, &str)],
    label_names: &[String],
    group_names: &std::collections::HashMap<String, String>,
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.given_name = Some(alpha.to_string());
                    first.family_name = None;
                    first.honorific_suffix = Some(numeric.to_string());
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.given_name = first.family_name.take();
                    first.family_name = None;
                    let new_given = first.given_name.as_deref().unwrap_or("");
                    first.unstructured_name = Some(new_given.to_string());
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.given_name = None;
                    first.family_name = None;
                    first.unstructured_name = None;
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.given_name = Some(new_name.clone());
                    first.family_name = None;
                    first.unstructured_name = Some(new_name.clone());
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.honorific_suffix = Some(new_suffix.clone());
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
        'e' => {
            interactive_edit_contact(hub, person, user_groups, label_names, group_names).await?;
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
    group_names: &std::collections::HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    match prompt_family_name_fix_action()? {
        'x' => {
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.family_name = None;
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.family_name = Some(new_name.clone());
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
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.honorific_suffix = Some(new_suffix.clone());
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
        'e' => {
            interactive_edit_contact(hub, person, user_groups, label_names, group_names).await?;
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_suffix_regexp(&hub, &contacts, &config.check_contact_suffix_regexp, &ctx).await?;
    Ok(())
}

const DEFAULT_SUFFIX_REGEX: &str = r"^[1-9]\d*$";

async fn check_suffix_regexp(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    config: &crate::helpers::NameRegexpConfig,
    ctx: &CheckContext<'_>,
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
            if !ctx.quiet {
                if count == 0
                    && let Some(header) = ctx.header {
                        println!("=== {} ===", header);
                    }
                println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

                if ctx.fix && !ctx.dry_run {
                    interactive_suffix_fix(hub, person, suffix, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                }
            }
            count += 1;
        }
    }
    if !ctx.quiet && count > 0 && ctx.header.is_some() { println!(); }
    Ok(count)
}

async fn interactive_suffix_fix(
    hub: &HubType,
    person: &google_people1::api::Person,
    suffix: &str,
    user_groups: &[(&str, &str)],
    label_names: &[String],
    group_names: &std::collections::HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    match prompt_fix_action(suffix)? {
        'r' => {
            let new_suffix = prompt_new_name(suffix)?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.honorific_suffix = Some(new_suffix.clone());
                }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Renamed suffix to \"{}\"", new_suffix);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'e' => {
            interactive_edit_contact(hub, person, user_groups, label_names, group_names).await?;
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

pub async fn cmd_check_contact_family_name_regexp(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_family_name_regexp(&hub, &contacts, &config.check_contact_family_name_regexp, &ctx).await?;
    Ok(())
}

async fn check_family_name_regexp(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    config: &crate::helpers::NameRegexpConfig,
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let pattern = match &config.allow {
        Some(p) => p,
        None => {
            if !ctx.quiet {
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
            if !ctx.quiet {
                if count == 0
                    && let Some(header) = ctx.header {
                        println!("=== {} ===", header);
                    }
                println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

                if ctx.fix && !ctx.dry_run {
                    interactive_family_name_fix(hub, person, family, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                }
            }
            count += 1;
        }
    }
    if !ctx.quiet && count > 0 && ctx.header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_no_given_name(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?
    };
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_no_given_name(&hub, &contacts, &ctx).await?;
    Ok(())
}

async fn check_no_given_name(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let names = person.names.as_ref().and_then(|n| n.first());
        let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
        let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");

        if !given.is_empty() || family.is_empty() {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

            if ctx.fix && !ctx.dry_run {
                interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
            }
        }
        count += 1;
    }
    if !ctx.quiet && count > 0 && ctx.header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_no_identity(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?
    };
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_no_identity(&hub, &contacts, &ctx).await?;
    Ok(())
}

async fn check_no_identity(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let has_given = person_has_given_name(person);
        let has_company = person.organizations.as_ref()
            .and_then(|orgs| orgs.first())
            .and_then(|o| o.name.as_deref())
            .is_some_and(|c| !c.is_empty());

        if has_given || has_company {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

            if ctx.fix && !ctx.dry_run {
                interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
            }
        }
        count += 1;
    }
    if !ctx.quiet && count > 0 && ctx.header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_name_is_company(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    if config.check_contact_name_is_company.companies.is_empty() {
        eprintln!("No [check-contact-name-is-company] companies configured in config.toml.");
        eprintln!("Add a section like:");
        eprintln!("  [check-contact-name-is-company]");
        eprintln!("  companies = [\"Google\", \"Microsoft\"]");
        return Ok(());
    }
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
        fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?
    };
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_name_is_company(&hub, &contacts, &config.check_contact_name_is_company.companies, &ctx).await?;
    Ok(())
}

async fn check_name_is_company(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    companies: &[String],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let company_set: std::collections::HashSet<String> = companies.iter()
        .map(|c| c.to_lowercase())
        .collect();

    let mut count = 0;
    for person in contacts {
        let names = person.names.as_ref().and_then(|n| n.first());
        let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
        let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");

        let given_match = !given.is_empty() && company_set.contains(&given.to_lowercase());
        let family_match = !family.is_empty() && company_set.contains(&family.to_lowercase());

        if !given_match && !family_match {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));
        }
        count += 1;

        if ctx.fix && !ctx.dry_run {
            interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
        }
    }

    if !ctx.quiet && count > 0
        && ctx.header.is_some() {
            println!();
        }

    Ok(count)
}

pub async fn cmd_check_contact_company_known(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    if config.check_contact_name_is_company.companies.is_empty() && !fix {
        eprintln!("No companies configured in config.toml.");
        eprintln!("Add companies to the [check-contact-name-is-company] section:");
        eprintln!("  [check-contact-name-is-company]");
        eprintln!("  companies = [\"Google\", \"Microsoft\"]");
        eprintln!("Or run with --fix to interactively add unknown companies.");
        return Ok(());
    }
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
        fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?
    };
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_company_known(&hub, &contacts, &config.check_contact_name_is_company.companies, &ctx).await?;
    Ok(())
}

async fn check_company_known(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    companies: &[String],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut company_set: std::collections::HashSet<String> = companies.iter()
        .map(|c| c.to_lowercase())
        .collect();

    let mut count = 0;
    for person in contacts {
        let org_name = person.organizations.as_ref()
            .and_then(|orgs| orgs.first())
            .and_then(|o| o.name.as_deref())
            .unwrap_or("");

        if org_name.is_empty() || company_set.contains(&org_name.to_lowercase()) {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

            if ctx.fix && !ctx.dry_run {
                use std::io::Write;
                loop {
                    eprint!("  [a]dd company to config / [e]dit contact / [s]kip? ");
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('a') => {
                            add_company_to_config(org_name)?;
                            company_set.insert(org_name.to_lowercase());
                            eprintln!("  Added \"{}\" to config.", org_name);
                            break;
                        }
                        Some('e') => {
                            interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                            break;
                        }
                        Some('s') => {
                            eprintln!("  Skipped.");
                            break;
                        }
                        _ => eprintln!("  Invalid choice. Enter a, e, or s."),
                    }
                }
            }
        }
        count += 1;
    }

    if !ctx.quiet && count > 0 && ctx.header.is_some() {
        println!();
    }

    Ok(count)
}

fn add_company_to_config(company: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = load_config();
    if !config.check_contact_name_is_company.companies.iter()
        .any(|c| c.eq_ignore_ascii_case(company))
    {
        config.check_contact_name_is_company.companies.push(company.to_string());
    }
    config.check_contact_name_is_company.companies.sort_by_key(|a| a.to_lowercase());
    save_company_list(&config.check_contact_name_is_company.companies)?;
    Ok(())
}

fn save_company_list(companies: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    let content = std::fs::read_to_string(&path)?;

    // Parse the TOML, update the companies list, and write back
    let mut doc = content.parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    let table = doc.entry("check-contact-name-is-company")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let arr = companies.iter()
        .map(|c| toml_edit::Value::from(c.as_str()))
        .collect::<toml_edit::Array>();
    table["companies"] = toml_edit::value(arr);

    std::fs::write(&path, doc.to_string())?;
    Ok(())
}

pub async fn cmd_check_contact_given_name_known(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_given_name_known(&hub, &contacts, &config.check_contact_given_name_known.names, &ctx).await?;
    Ok(())
}

async fn check_given_name_known(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    names: &[String],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut name_set: std::collections::HashSet<String> = names.iter()
        .map(|n| n.to_lowercase())
        .collect();

    let mut count = 0;
    for person in contacts {
        let given = person.names.as_ref()
            .and_then(|n| n.first())
            .and_then(|n| n.given_name.as_deref())
            .unwrap_or("");

        if given.is_empty() || name_set.contains(&given.to_lowercase()) {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

            if ctx.fix && !ctx.dry_run {
                use std::io::Write;
                loop {
                    eprint!("  [a]dd name to config / [e]dit contact / [s]kip? ");
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('a') => {
                            add_given_name_to_config(given)?;
                            name_set.insert(given.to_lowercase());
                            eprintln!("  Added \"{}\" to config.", given);
                            break;
                        }
                        Some('e') => {
                            interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                            break;
                        }
                        Some('s') => {
                            eprintln!("  Skipped.");
                            break;
                        }
                        _ => eprintln!("  Invalid choice. Enter a, e, or s."),
                    }
                }
            }
        }
        count += 1;
    }

    if !ctx.quiet && count > 0 && ctx.header.is_some() {
        println!();
    }

    Ok(count)
}

fn add_given_name_to_config(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = load_config();
    if !config.check_contact_given_name_known.names.iter()
        .any(|n| n.eq_ignore_ascii_case(name))
    {
        config.check_contact_given_name_known.names.push(name.to_string());
    }
    config.check_contact_given_name_known.names.sort_by_key(|a| a.to_lowercase());
    save_given_name_list(&config.check_contact_given_name_known.names)?;
    Ok(())
}

fn save_given_name_list(names: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let path = config_path();
    let content = std::fs::read_to_string(&path)?;

    let mut doc = content.parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("Failed to parse config: {}", e))?;

    let table = doc.entry("check-contact-given-name-known")
        .or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    let arr = names.iter()
        .map(|n| toml_edit::Value::from(n.as_str()))
        .collect::<toml_edit::Array>();
    table["names"] = toml_edit::value(arr);

    std::fs::write(&path, doc.to_string())?;
    Ok(())
}

pub async fn cmd_check_contact_displayname_duplicate(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_name_duplicate(&hub, &contacts, &ctx).await?;
    Ok(())
}

async fn check_name_duplicate(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
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

    if !ctx.quiet && !duplicates.is_empty() {
        if let Some(header) = ctx.header {
            println!("=== {} ({}) ===", header, count);
        }

        for (name, group) in &duplicates {
            println!("{}\"{}\" ({} contacts):", ctx.prefix, name, group.len());
            for person in *group {
                println!("{}  - {}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));
            }

            if ctx.fix && !ctx.dry_run {
                eprint!("{}  [n]umber all / fix [i]ndividually / [s]kip? ", ctx.prefix);
                use std::io::Write;
                std::io::stderr().flush()?;
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                match input.trim() {
                    "n" => {
                        for (i, person) in group.iter().enumerate() {
                            let suffix = (i + 1).to_string();
                            let resource_name = person.resource_name.as_deref()
                                .ok_or("Contact missing resource name")?;
                            let mut updated = (*person).clone();
                            match updated.names {
                                Some(ref mut names) if !names.is_empty() => {
                                    names[0].honorific_suffix = Some(suffix.clone());
                                }
                                _ => {
                                    updated.names = Some(vec![google_people1::api::Name {
                                        honorific_suffix: Some(suffix.clone()),
                                        ..Default::default()
                                    }]);
                                }
                            }
                            hub.people()
                                .update_contact(updated, resource_name)
                                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                                .doit()
                                .await?;
                            eprintln!("{}  {} -> suffix \"{}\"", ctx.prefix, person_display_name(person), suffix);
                            tokio::time::sleep(MUTATE_DELAY).await;
                        }
                    }
                    "i" => {
                        for person in *group {
                            eprintln!("{}  Fix duplicate: {}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));
                            let display = person_display_name(person);
                            interactive_name_duplicate_fix(hub, person, &display, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                        }
                    }
                    _ => {
                        eprintln!("{}  Skipped.", ctx.prefix);
                    }
                }
            }
        }

        if ctx.header.is_some() {
            println!();
        }
    }

    Ok(count)
}

async fn interactive_name_duplicate_fix(
    hub: &HubType,
    person: &google_people1::api::Person,
    name: &str,
    user_groups: &[(&str, &str)],
    label_names: &[String],
    group_names: &std::collections::HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?;

    match prompt_fix_action(name)? {
        'r' => {
            let new_name = prompt_new_name(name)?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names
                && let Some(first) = names.first_mut() {
                    first.given_name = Some(new_name.clone());
                    first.family_name = None;
                    first.unstructured_name = Some(new_name.clone());
                }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("  Renamed to \"{}\"", new_name);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
        'e' => {
            interactive_edit_contact(hub, person, user_groups, label_names, group_names).await?;
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

pub async fn cmd_check_phone_countrycode(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers", "emailAddresses", "nicknames", "memberships"]).await?;
    let country = country.to_string();
    let empty_group_names = std::collections::HashMap::new();
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &[], label_names: &[], group_names: &empty_group_names };
    check_phone_issues(
        &hub, &contacts,
        |v| is_fixable_phone(v) && !has_country_code(v),
        move |v| add_country_code(v, &country),
        &ctx,
    ).await?;
    Ok(())
}

pub async fn cmd_check_phone_format(fix: bool, dry_run: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers", "emailAddresses", "nicknames", "memberships"]).await?;
    let country_owned = country.to_string();
    let empty_group_names = std::collections::HashMap::new();
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &[], label_names: &[], group_names: &empty_group_names };
    check_phone_issues(
        &hub, &contacts,
        |v| is_fixable_phone(v) && !is_correct_phone_format(v),
        move |v| fix_phone_format(v, &country_owned),
        &ctx,
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
                    if count == 0
                        && let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    println!("{}{}", prefix, format_person_line(person, None));
                    for phone in &dupes {
                        println!("{}  duplicate: {}", prefix, phone);
                    }
                }
                count += dupes.len();

                if fix && !dry_run && !quiet {
                    if prompt_yes_no(&format!("  Remove duplicate(s) \"{}\"?", dupes.join(", ")))? {
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
                if count == 0
                    && let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                println!("{}{}", prefix, format_person_line(person, None));
                for email in emails {
                    if let Some(val) = email.value.as_deref() {
                        if !is_valid_email(val) {
                            println!("{}  {} (invalid)", prefix, val);
                        } else if val != val.to_lowercase().as_str() {
                            if fix || dry_run {
                                println!("{}  {} -> {}", prefix, val, val.to_lowercase());
                            } else {
                                println!("{}  {} (uppercase)", prefix, val);
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
                    if prompt_yes_no("  Fix?")? {
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

async fn check_no_label(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if !has_user_label(person) {
            if !ctx.quiet {
                if count == 0
                    && let Some(header) = ctx.header {
                        println!("=== {} ===", header);
                    }
                println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));
            }
            count += 1;

            if ctx.fix && !ctx.quiet {

                if ctx.dry_run {
                    eprintln!("(dry-run) would prompt for action\n");
                    continue;
                }
                let resource_name = match person.resource_name.as_deref() {
                    Some(rn) => rn,
                    None => continue,
                };
                use std::io::Write;
                loop {
                    eprint!("[l]abel / [e]dit / [d]elete / [s]kip: ");
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('l') => {
                            print_person_details(person, None);
                            println!("{}", "-".repeat(60));
                            if let Some(group_rn) = prompt_label_autocomplete(hub, ctx.label_names, ctx.user_groups).await? {
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
                        Some('e') => {
                            interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
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
                        _ => eprintln!("  Invalid choice. Enter l, e, d, or s."),
                    }
                }
            }
        }
    }
    if !ctx.quiet && count > 0 && ctx.header.is_some() { println!(); }
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers", "emailAddresses", "nicknames", "memberships"]).await?;
    check_duplicate_phones(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_email(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
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
                    if count == 0
                        && let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    println!("{}{}", prefix, format_person_line(person, None));
                    for email in &dupes {
                        println!("{}  duplicate: {}", prefix, email);
                    }
                }
                count += dupes.len();

                if fix && !dry_run && !quiet {
                    if prompt_yes_no(&format!("  Remove duplicate(s) \"{}\"?", dupes.join(", ")))? {
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
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
                    if count == 0
                        && let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    println!("{}{}", prefix, format_person_line(person, None));
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers", "emailAddresses", "nicknames", "memberships"]).await?;
    check_phone_label_missing(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_phone_label_english(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(nums) = &person.phone_numbers {
            let has_non_english = nums.iter().any(|pn| {
                let label = get_phone_label(pn);
                !label.is_empty() && !label.is_ascii()
            });
            if has_non_english {
                if !quiet {
                    if count == 0
                        && let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    println!("{}{}", prefix, format_person_line(person, None));
                }
                count += nums.iter().filter(|pn| {
                    let label = get_phone_label(pn);
                    !label.is_empty() && !label.is_ascii()
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers", "emailAddresses", "nicknames", "memberships"]).await?;
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
            if !label.is_empty() && !label.is_ascii() {
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
        fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?
    };

    let (user_groups_owned, label_names, group_names) = if fix {
        let all_groups = fetch_all_contact_groups(&hub).await?;
        let gn = build_group_name_map(&all_groups);
        let ug: Vec<(String, String)> = all_groups.iter()
            .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
            .filter_map(|g| {
                let name = g.name.as_deref()?;
                let rn = g.resource_name.as_deref()?;
                Some((name.to_string(), rn.to_string()))
            })
            .collect();
        let ln: Vec<String> = ug.iter().map(|(name, _)| name.clone()).collect();
        (ug, ln, gn)
    } else {
        (vec![], vec![], std::collections::HashMap::new())
    };
    let user_groups: Vec<(&str, &str)> = user_groups_owned.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();

    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_no_label(&hub, &contacts, &ctx).await?;
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
                    let new_group = google_people1::api::ContactGroup {
                        name: Some(trimmed.to_string()),
                        ..Default::default()
                    };
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

const EDIT_PERSON_FIELDS: &[&str] = &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"];

fn get_name_field<'a>(person: &'a google_people1::api::Person, field: &str) -> &'a str {
    let names = person.names.as_ref().and_then(|n| n.first());
    match field {
        "given_name" => names.and_then(|n| n.given_name.as_deref()).unwrap_or(""),
        "family_name" => names.and_then(|n| n.family_name.as_deref()).unwrap_or(""),
        "middle_name" => names.and_then(|n| n.middle_name.as_deref()).unwrap_or(""),
        "honorific_prefix" => names.and_then(|n| n.honorific_prefix.as_deref()).unwrap_or(""),
        "honorific_suffix" => names.and_then(|n| n.honorific_suffix.as_deref()).unwrap_or(""),
        _ => "",
    }
}

fn get_org_field<'a>(person: &'a google_people1::api::Person, field: &str) -> &'a str {
    let org = person.organizations.as_ref().and_then(|o| o.first());
    match field {
        "name" => org.and_then(|o| o.name.as_deref()).unwrap_or(""),
        "title" => org.and_then(|o| o.title.as_deref()).unwrap_or(""),
        "department" => org.and_then(|o| o.department.as_deref()).unwrap_or(""),
        _ => "",
    }
}

fn display_edit_menu(person: &google_people1::api::Person, group_names: &std::collections::HashMap<String, String>) {
    let names = person.names.as_ref().and_then(|n| n.first());
    let org = person.organizations.as_ref().and_then(|o| o.first());

    eprintln!();
    eprintln!("  ---- Name fields ----");
    eprintln!("   1) Prefix        : {}", names.and_then(|n| n.honorific_prefix.as_deref()).unwrap_or(""));
    eprintln!("   2) Given name    : {}", names.and_then(|n| n.given_name.as_deref()).unwrap_or(""));
    eprintln!("   3) Middle name   : {}", names.and_then(|n| n.middle_name.as_deref()).unwrap_or(""));
    eprintln!("   4) Family name   : {}", names.and_then(|n| n.family_name.as_deref()).unwrap_or(""));
    eprintln!("   5) Suffix        : {}", names.and_then(|n| n.honorific_suffix.as_deref()).unwrap_or(""));
    eprintln!("   6) Nickname      : {}", person.nicknames.as_ref().and_then(|n| n.first()).and_then(|n| n.value.as_deref()).unwrap_or(""));
    eprintln!("  ---- Organization ----");
    eprintln!("   7) Company       : {}", org.and_then(|o| o.name.as_deref()).unwrap_or(""));
    eprintln!("   8) Title         : {}", org.and_then(|o| o.title.as_deref()).unwrap_or(""));
    eprintln!("   9) Department    : {}", org.and_then(|o| o.department.as_deref()).unwrap_or(""));
    eprintln!("  ---- Phones ----");
    if let Some(phones) = &person.phone_numbers {
        for (i, pn) in phones.iter().enumerate() {
            let val = pn.value.as_deref().unwrap_or("");
            let label = get_phone_label(pn);
            if label.is_empty() { eprintln!("       {}. {}", i + 1, val); }
            else { eprintln!("       {}. {} [{}]", i + 1, val, label); }
        }
    }
    eprintln!("  10) Edit phones");
    eprintln!("  ---- Emails ----");
    if let Some(emails) = &person.email_addresses {
        for (i, e) in emails.iter().enumerate() {
            let val = e.value.as_deref().unwrap_or("");
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
            if t.is_empty() { eprintln!("       {}. {}", i + 1, val); }
            else { eprintln!("       {}. {} [{}]", i + 1, val, t); }
        }
    }
    eprintln!("  11) Edit emails");
    eprintln!("  ---- Labels ----");
    let labels = person_labels(person, group_names);
    if !labels.is_empty() {
        eprintln!("       {}", labels.join(", "));
    }
    eprintln!("  12) Add label");
    eprintln!("  13) Remove label");
    eprintln!("  ---- Actions ----");
    eprintln!("   d) Delete contact");
    eprintln!("   s) Skip (done editing)");
}

async fn edit_simple_name_field(
    hub: &HubType,
    current: &mut google_people1::api::Person,
    resource_name: &str,
    field: &str,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let cur_val = get_name_field(current, field);
    eprint!("  {} [{}] (new value / - to clear / Enter to skip): ", label, cur_val);
    std::io::stderr().flush()?;
    let mut val = String::new();
    std::io::stdin().read_line(&mut val)?;
    let val = val.trim();
    if val.is_empty() {
        eprintln!("  Unchanged.");
        return Ok(());
    }
    let new_val = if val == "-" { None } else { Some(val.to_string()) };
    let mut updated = current.clone();
    if updated.names.is_none() {
        updated.names = Some(vec![google_people1::api::Name::default()]);
    }
    if let Some(ref mut names) = updated.names
        && let Some(first) = names.first_mut() {
            match field {
                "given_name" => first.given_name = new_val.clone(),
                "family_name" => first.family_name = new_val.clone(),
                "middle_name" => first.middle_name = new_val.clone(),
                "honorific_prefix" => first.honorific_prefix = new_val.clone(),
                "honorific_suffix" => first.honorific_suffix = new_val.clone(),
                _ => {}
            }
            // Rebuild unstructured_name for given/family changes
            let g = first.given_name.as_deref().unwrap_or("");
            let f = first.family_name.as_deref().unwrap_or("");
            let combined = [g, f].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ");
            first.unstructured_name = if combined.is_empty() { None } else { Some(combined) };
        }
    let (_, refreshed) = hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["names"]))
        .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
        .doit()
        .await?;
    *current = refreshed;
    match new_val {
        Some(v) => eprintln!("  Set {} to \"{}\"", label.to_lowercase(), v),
        None => eprintln!("  Cleared {}.", label.to_lowercase()),
    }
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

async fn edit_org_field(
    hub: &HubType,
    current: &mut google_people1::api::Person,
    resource_name: &str,
    field: &str,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let cur_val = get_org_field(current, field);
    eprint!("  {} [{}] (new value / - to clear / Enter to skip): ", label, cur_val);
    std::io::stderr().flush()?;
    let mut val = String::new();
    std::io::stdin().read_line(&mut val)?;
    let val = val.trim();
    if val.is_empty() {
        eprintln!("  Unchanged.");
        return Ok(());
    }
    let mut updated = current.clone();
    if field == "name" && val == "-" {
        // Clearing company clears the whole org
        updated.organizations = Some(vec![]);
    } else {
        if updated.organizations.is_none() || updated.organizations.as_ref().is_some_and(|o| o.is_empty()) {
            updated.organizations = Some(vec![google_people1::api::Organization::default()]);
        }
        if field == "name" {
            // Setting company replaces the org but preserves other fields
            if let Some(ref mut orgs) = updated.organizations
                && let Some(first) = orgs.first_mut() {
                    first.name = Some(val.to_string());
                }
        } else if let Some(ref mut orgs) = updated.organizations
            && let Some(first) = orgs.first_mut() {
                let new_val = if val == "-" { None } else { Some(val.to_string()) };
                match field {
                    "title" => first.title = new_val,
                    "department" => first.department = new_val,
                    _ => {}
                }
            }
    }
    let (_, refreshed) = hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["organizations"]))
        .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
        .doit()
        .await?;
    *current = refreshed;
    if val == "-" {
        eprintln!("  Cleared {}.", label.to_lowercase());
    } else {
        eprintln!("  Set {} to \"{}\"", label.to_lowercase(), val);
    }
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

async fn edit_nickname(
    hub: &HubType,
    current: &mut google_people1::api::Person,
    resource_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let cur_val = current.nicknames.as_ref()
        .and_then(|n| n.first())
        .and_then(|n| n.value.as_deref())
        .unwrap_or("");
    eprint!("  Nickname [{}] (new value / - to clear / Enter to skip): ", cur_val);
    std::io::stderr().flush()?;
    let mut val = String::new();
    std::io::stdin().read_line(&mut val)?;
    let val = val.trim();
    if val.is_empty() {
        eprintln!("  Unchanged.");
        return Ok(());
    }
    let mut updated = current.clone();
    if val == "-" {
        updated.nicknames = Some(vec![]);
    } else {
        updated.nicknames = Some(vec![google_people1::api::Nickname {
            value: Some(val.to_string()),
            ..Default::default()
        }]);
    }
    let (_, refreshed) = hub.people()
        .update_contact(updated, resource_name)
        .update_person_fields(FieldMask::new::<&str>(&["nicknames"]))
        .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
        .doit()
        .await?;
    *current = refreshed;
    if val == "-" {
        eprintln!("  Cleared nickname.");
    } else {
        eprintln!("  Set nickname to \"{}\"", val);
    }
    tokio::time::sleep(MUTATE_DELAY).await;
    Ok(())
}

async fn edit_phones(
    hub: &HubType,
    current: &mut google_people1::api::Person,
    resource_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let phones: Vec<String> = current.phone_numbers.as_ref()
        .map(|nums| nums.iter().enumerate().map(|(i, pn)| {
            let val = pn.value.as_deref().unwrap_or("");
            let label = get_phone_label(pn);
            if label.is_empty() { format!("  {}: {}", i + 1, val) }
            else { format!("  {}: {} [{}]", i + 1, val, label) }
        }).collect())
        .unwrap_or_default();
    if phones.is_empty() {
        eprintln!("  No phone numbers.");
    } else {
        for p in &phones { eprintln!("{}", p); }
    }
    eprint!("  [a]dd / [r]emove #N / [e]dit #N / [b]ack? ");
    std::io::stderr().flush()?;
    let mut sub = String::new();
    std::io::stdin().read_line(&mut sub)?;
    let sub = sub.trim();
    if sub.starts_with('a') {
        eprint!("  New phone number: ");
        std::io::stderr().flush()?;
        let mut num = String::new();
        std::io::stdin().read_line(&mut num)?;
        let num = num.trim();
        if num.is_empty() { return Ok(()); }
        let label = prompt_phone_label_fix(&person_display_name(current))?;
        let mut updated = current.clone();
        let mut pn = google_people1::api::PhoneNumber {
            value: Some(num.to_string()),
            ..Default::default()
        };
        if let Some(l) = label {
            pn.type_ = Some(l);
        }
        updated.phone_numbers.get_or_insert_with(Vec::new).push(pn);
        let (_, refreshed) = hub.people()
            .update_contact(updated, resource_name)
            .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
            .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
            .doit()
            .await?;
        *current = refreshed;
        eprintln!("  Added phone \"{}\"", num);
        tokio::time::sleep(MUTATE_DELAY).await;
    } else if let Some(rest) = sub.strip_prefix('r') {
        if let Ok(idx) = rest.trim().parse::<usize>() {
            let mut updated = current.clone();
            if let Some(ref mut nums) = updated.phone_numbers {
                if idx >= 1 && idx <= nums.len() {
                    nums.remove(idx - 1);
                    let (_, refreshed) = hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                        .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
                        .doit()
                        .await?;
                    *current = refreshed;
                    eprintln!("  Removed phone #{}", idx);
                    tokio::time::sleep(MUTATE_DELAY).await;
                } else {
                    eprintln!("  Invalid index.");
                }
            }
        } else {
            eprintln!("  Usage: r1, r2, etc.");
        }
    } else if let Some(rest) = sub.strip_prefix('e') {
        if let Ok(idx) = rest.trim().parse::<usize>() {
            let nums_len = current.phone_numbers.as_ref().map_or(0, |n| n.len());
            if idx >= 1 && idx <= nums_len {
                let cur_phone = current.phone_numbers.as_ref().unwrap()[idx - 1].value.as_deref().unwrap_or("");
                eprint!("  Phone [{}]: ", cur_phone);
                std::io::stderr().flush()?;
                let mut val = String::new();
                std::io::stdin().read_line(&mut val)?;
                let val = val.trim();
                if val.is_empty() { return Ok(()); }
                let mut updated = current.clone();
                if let Some(ref mut nums) = updated.phone_numbers {
                    nums[idx - 1].value = Some(val.to_string());
                }
                let (_, refreshed) = hub.people()
                    .update_contact(updated, resource_name)
                    .update_person_fields(FieldMask::new::<&str>(&["phoneNumbers"]))
                    .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
                    .doit()
                    .await?;
                *current = refreshed;
                eprintln!("  Updated phone #{} to \"{}\"", idx, val);
                tokio::time::sleep(MUTATE_DELAY).await;
            } else {
                eprintln!("  Invalid index.");
            }
        } else {
            eprintln!("  Usage: e1, e2, etc.");
        }
    }
    Ok(())
}

async fn edit_emails(
    hub: &HubType,
    current: &mut google_people1::api::Person,
    resource_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let emails: Vec<String> = current.email_addresses.as_ref()
        .map(|ems| ems.iter().enumerate().map(|(i, e)| {
            let val = e.value.as_deref().unwrap_or("");
            let t = e.formatted_type.as_deref().or(e.type_.as_deref()).unwrap_or("");
            if t.is_empty() { format!("  {}: {}", i + 1, val) }
            else { format!("  {}: {} [{}]", i + 1, val, t) }
        }).collect())
        .unwrap_or_default();
    if emails.is_empty() {
        eprintln!("  No email addresses.");
    } else {
        for e in &emails { eprintln!("{}", e); }
    }
    eprint!("  [a]dd / [r]emove #N / [e]dit #N / [b]ack? ");
    std::io::stderr().flush()?;
    let mut sub = String::new();
    std::io::stdin().read_line(&mut sub)?;
    let sub = sub.trim();
    if sub.starts_with('a') {
        eprint!("  New email: ");
        std::io::stderr().flush()?;
        let mut val = String::new();
        std::io::stdin().read_line(&mut val)?;
        let val = val.trim();
        if val.is_empty() { return Ok(()); }
        let mut updated = current.clone();
        let em = google_people1::api::EmailAddress {
            value: Some(val.to_string()),
            ..Default::default()
        };
        updated.email_addresses.get_or_insert_with(Vec::new).push(em);
        let (_, refreshed) = hub.people()
            .update_contact(updated, resource_name)
            .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
            .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
            .doit()
            .await?;
        *current = refreshed;
        eprintln!("  Added email \"{}\"", val);
        tokio::time::sleep(MUTATE_DELAY).await;
    } else if let Some(rest) = sub.strip_prefix('r') {
        if let Ok(idx) = rest.trim().parse::<usize>() {
            let mut updated = current.clone();
            if let Some(ref mut ems) = updated.email_addresses {
                if idx >= 1 && idx <= ems.len() {
                    ems.remove(idx - 1);
                    let (_, refreshed) = hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
                        .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
                        .doit()
                        .await?;
                    *current = refreshed;
                    eprintln!("  Removed email #{}", idx);
                    tokio::time::sleep(MUTATE_DELAY).await;
                } else {
                    eprintln!("  Invalid index.");
                }
            }
        } else {
            eprintln!("  Usage: r1, r2, etc.");
        }
    } else if let Some(rest) = sub.strip_prefix('e') {
        if let Ok(idx) = rest.trim().parse::<usize>() {
            let ems_len = current.email_addresses.as_ref().map_or(0, |e| e.len());
            if idx >= 1 && idx <= ems_len {
                let cur_email = current.email_addresses.as_ref().unwrap()[idx - 1].value.as_deref().unwrap_or("");
                eprint!("  Email [{}]: ", cur_email);
                std::io::stderr().flush()?;
                let mut val = String::new();
                std::io::stdin().read_line(&mut val)?;
                let val = val.trim();
                if val.is_empty() { return Ok(()); }
                let mut updated = current.clone();
                if let Some(ref mut ems) = updated.email_addresses {
                    ems[idx - 1].value = Some(val.to_string());
                }
                let (_, refreshed) = hub.people()
                    .update_contact(updated, resource_name)
                    .update_person_fields(FieldMask::new::<&str>(&["emailAddresses"]))
                    .person_fields(FieldMask::new::<&str>(EDIT_PERSON_FIELDS))
                    .doit()
                    .await?;
                *current = refreshed;
                eprintln!("  Updated email #{} to \"{}\"", idx, val);
                tokio::time::sleep(MUTATE_DELAY).await;
            } else {
                eprintln!("  Invalid index.");
            }
        } else {
            eprintln!("  Usage: e1, e2, etc.");
        }
    }
    Ok(())
}

async fn interactive_edit_contact(
    hub: &HubType,
    person: &google_people1::api::Person,
    user_groups: &[(&str, &str)],
    label_names: &[String],
    group_names: &std::collections::HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    let resource_name = person
        .resource_name
        .as_deref()
        .ok_or("Contact missing resource name")?
        .to_string();

    let mut current = person.clone();

    loop {
        display_edit_menu(&current, group_names);
        eprint!("  > ");
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "1" => edit_simple_name_field(hub, &mut current, &resource_name, "honorific_prefix", "Prefix").await?,
            "2" => edit_simple_name_field(hub, &mut current, &resource_name, "given_name", "Given name").await?,
            "3" => edit_simple_name_field(hub, &mut current, &resource_name, "middle_name", "Middle name").await?,
            "4" => edit_simple_name_field(hub, &mut current, &resource_name, "family_name", "Family name").await?,
            "5" => edit_simple_name_field(hub, &mut current, &resource_name, "honorific_suffix", "Suffix").await?,
            "6" => edit_nickname(hub, &mut current, &resource_name).await?,
            "7" => edit_org_field(hub, &mut current, &resource_name, "name", "Company").await?,
            "8" => edit_org_field(hub, &mut current, &resource_name, "title", "Title").await?,
            "9" => edit_org_field(hub, &mut current, &resource_name, "department", "Department").await?,
            "10" => edit_phones(hub, &mut current, &resource_name).await?,
            "11" => edit_emails(hub, &mut current, &resource_name).await?,
            "12" => {
                if let Some(group_rn) = prompt_label_autocomplete(hub, label_names, user_groups).await? {
                    let req = google_people1::api::ModifyContactGroupMembersRequest {
                        resource_names_to_add: Some(vec![resource_name.to_string()]),
                        resource_names_to_remove: None,
                    };
                    hub.contact_groups().members_modify(req, &group_rn).doit().await?;
                    eprintln!("  Assigned label.");
                    tokio::time::sleep(MUTATE_DELAY).await;
                }
            }
            "13" => {
                let labels = person_labels(&current, group_names);
                if labels.is_empty() {
                    eprintln!("  No labels to remove.");
                    continue;
                }
                for (i, l) in labels.iter().enumerate() {
                    eprintln!("  {}: {}", i + 1, l);
                }
                eprint!("  Remove label # (or Enter to cancel): ");
                std::io::stderr().flush()?;
                let mut val = String::new();
                std::io::stdin().read_line(&mut val)?;
                let val = val.trim();
                if val.is_empty() { continue; }
                if let Ok(idx) = val.parse::<usize>() {
                    if idx >= 1 && idx <= labels.len() {
                        let label_name = &labels[idx - 1];
                        if let Some((_, rn)) = user_groups.iter().find(|(name, _)| name == label_name) {
                            let req = google_people1::api::ModifyContactGroupMembersRequest {
                                resource_names_to_add: None,
                                resource_names_to_remove: Some(vec![resource_name.to_string()]),
                            };
                            hub.contact_groups().members_modify(req, rn).doit().await?;
                            eprintln!("  Removed label \"{}\"", label_name);
                            tokio::time::sleep(MUTATE_DELAY).await;
                        } else {
                            eprintln!("  Could not find label group.");
                        }
                    } else {
                        eprintln!("  Invalid index.");
                    }
                }
            }
            "d" => {
                if prompt_yes_no(&format!("Delete {}?", person_display_name(&current)))? {
                    hub.people().delete_contact(&resource_name).doit().await?;
                    eprintln!("  Deleted.");
                    tokio::time::sleep(MUTATE_DELAY).await;
                }
                break;
            }
            "s" | "" => {
                break;
            }
            _ => {
                eprintln!("  Invalid choice. Enter a number (1-13), d, or s.");
            }
        }
    }
    Ok(())
}

pub async fn cmd_check_contact_type(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_contact_type(&hub, &contacts, &ctx).await?;
    Ok(())
}

const TYPE_PERSON_LABEL: &str = "TypePerson";
const TYPE_COMPANY_LABEL: &str = "TypeCompany";

fn person_type_labels(person: &google_people1::api::Person, group_names: &std::collections::HashMap<String, String>) -> (bool, bool) {
    let labels = person_labels(person, group_names);
    let has_person = labels.iter().any(|l| l == TYPE_PERSON_LABEL);
    let has_company = labels.iter().any(|l| l == TYPE_COMPANY_LABEL);
    (has_person, has_company)
}

async fn ensure_type_labels_exist(
    hub: &HubType,
    user_groups: &[(&str, &str)],
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let person_rn = if let Some((_, rn)) = user_groups.iter().find(|(name, _)| *name == TYPE_PERSON_LABEL) {
        rn.to_string()
    } else {
        let new_group = google_people1::api::ContactGroup {
            name: Some(TYPE_PERSON_LABEL.to_string()),
            ..Default::default()
        };
        let req = google_people1::api::CreateContactGroupRequest {
            contact_group: Some(new_group),
            read_group_fields: None,
        };
        let (_, created) = hub.contact_groups().create(req).doit().await?;
        eprintln!("Created label \"{}\"", TYPE_PERSON_LABEL);
        created.resource_name.ok_or("Created group missing resource name")?
    };

    let company_rn = if let Some((_, rn)) = user_groups.iter().find(|(name, _)| *name == TYPE_COMPANY_LABEL) {
        rn.to_string()
    } else {
        let new_group = google_people1::api::ContactGroup {
            name: Some(TYPE_COMPANY_LABEL.to_string()),
            ..Default::default()
        };
        let req = google_people1::api::CreateContactGroupRequest {
            contact_group: Some(new_group),
            read_group_fields: None,
        };
        let (_, created) = hub.contact_groups().create(req).doit().await?;
        eprintln!("Created label \"{}\"", TYPE_COMPANY_LABEL);
        created.resource_name.ok_or("Created group missing resource name")?
    };

    Ok((person_rn, company_rn))
}

async fn check_contact_type(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    let mut type_rns: Option<(String, String)> = None;

    for person in contacts {
        let (has_person, has_company) = person_type_labels(person, ctx.group_names);

        let issue = if has_person && has_company {
            Some("has both TypePerson and TypeCompany")
        } else if !has_person && !has_company {
            Some("missing TypePerson or TypeCompany")
        } else {
            None
        };

        let Some(issue) = issue else { continue; };

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{} ({})", ctx.prefix, format_person_line(person, Some(ctx.group_names)), issue);

            if ctx.fix && !ctx.dry_run {
                let resource_name = match person.resource_name.as_deref() {
                    Some(rn) => rn,
                    None => { count += 1; continue; }
                };

                // Lazily ensure labels exist
                if type_rns.is_none() {
                    type_rns = Some(ensure_type_labels_exist(hub, ctx.user_groups).await?);
                }
                let (ref person_rn, ref company_rn) = *type_rns.as_ref().unwrap();

                use std::io::Write;
                if has_person && has_company {
                    // Both — need to remove one
                    loop {
                        eprint!("  Remove [p]erson / remove [c]ompany / [e]dit / [s]kip? ");
                        std::io::stderr().flush()?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        match input.trim().chars().next() {
                            Some('p') => {
                                let req = google_people1::api::ModifyContactGroupMembersRequest {
                                    resource_names_to_add: None,
                                    resource_names_to_remove: Some(vec![resource_name.to_string()]),
                                };
                                hub.contact_groups().members_modify(req, person_rn).doit().await?;
                                eprintln!("  Removed TypePerson.");
                                tokio::time::sleep(MUTATE_DELAY).await;
                                break;
                            }
                            Some('c') => {
                                let req = google_people1::api::ModifyContactGroupMembersRequest {
                                    resource_names_to_add: None,
                                    resource_names_to_remove: Some(vec![resource_name.to_string()]),
                                };
                                hub.contact_groups().members_modify(req, company_rn).doit().await?;
                                eprintln!("  Removed TypeCompany.");
                                tokio::time::sleep(MUTATE_DELAY).await;
                                break;
                            }
                            Some('e') => {
                                interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                                break;
                            }
                            Some('s') => {
                                eprintln!("  Skipped.");
                                break;
                            }
                            _ => eprintln!("  Invalid choice. Enter p, c, e, or s."),
                        }
                    }
                } else {
                    // Neither — need to assign one
                    loop {
                        eprint!("  Assign [p]erson / assign [c]ompany / [e]dit / [s]kip? ");
                        std::io::stderr().flush()?;
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;
                        match input.trim().chars().next() {
                            Some('p') => {
                                let req = google_people1::api::ModifyContactGroupMembersRequest {
                                    resource_names_to_add: Some(vec![resource_name.to_string()]),
                                    resource_names_to_remove: None,
                                };
                                hub.contact_groups().members_modify(req, person_rn).doit().await?;
                                eprintln!("  Assigned TypePerson.");
                                tokio::time::sleep(MUTATE_DELAY).await;
                                break;
                            }
                            Some('c') => {
                                let req = google_people1::api::ModifyContactGroupMembersRequest {
                                    resource_names_to_add: Some(vec![resource_name.to_string()]),
                                    resource_names_to_remove: None,
                                };
                                hub.contact_groups().members_modify(req, company_rn).doit().await?;
                                eprintln!("  Assigned TypeCompany.");
                                tokio::time::sleep(MUTATE_DELAY).await;
                                break;
                            }
                            Some('e') => {
                                interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                                break;
                            }
                            Some('s') => {
                                eprintln!("  Skipped.");
                                break;
                            }
                            _ => eprintln!("  Invalid choice. Enter p, c, e, or s."),
                        }
                    }
                }
            }
        }
        count += 1;
    }

    if !ctx.quiet && count > 0 && ctx.header.is_some() {
        println!();
    }

    Ok(count)
}

pub async fn cmd_move_given_name_to_company(name: &str, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);

    let matching: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        p.names.as_ref()
            .and_then(|n| n.first())
            .and_then(|n| n.given_name.as_deref())
            .is_some_and(|g| g.eq_ignore_ascii_case(name))
    }).collect();

    if matching.is_empty() {
        eprintln!("No contacts found with given name \"{}\"", name);
        return Ok(());
    }

    eprintln!("Found {} contact(s) with given name \"{}\":", matching.len(), name);
    for person in &matching {
        println!("  {}", format_person_line(person, Some(&group_names)));
    }

    if dry_run {
        eprintln!("(dry-run) Would move given name to company for all above contacts.");
        return Ok(());
    }

    if !prompt_yes_no(&format!("Move given name \"{}\" to company for all {} contact(s)?", name, matching.len()))? {
        eprintln!("Cancelled.");
        return Ok(());
    }

    for person in &matching {
        let resource_name = match person.resource_name.as_deref() {
            Some(rn) => rn,
            None => continue,
        };
        let given = person.names.as_ref()
            .and_then(|n| n.first())
            .and_then(|n| n.given_name.as_deref())
            .unwrap_or("");

        // Clear given name and unstructured_name
        let mut updated = (*person).clone();
        if let Some(ref mut names) = updated.names
            && let Some(first) = names.first_mut() {
                first.given_name = None;
                first.unstructured_name = None;
            }
        let (_, refreshed) = hub.people()
            .update_contact(updated, resource_name)
            .update_person_fields(FieldMask::new::<&str>(&["names"]))
            .person_fields(FieldMask::new::<&str>(&["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]))
            .doit()
            .await?;
        tokio::time::sleep(MUTATE_DELAY).await;

        // Set company
        let mut updated2 = refreshed;
        updated2.organizations = Some(vec![google_people1::api::Organization {
            name: Some(given.to_string()),
            ..Default::default()
        }]);
        hub.people()
            .update_contact(updated2, resource_name)
            .update_person_fields(FieldMask::new::<&str>(&["organizations"]))
            .doit()
            .await?;

        eprintln!("  Moved \"{}\" -> company for {}", given, person_display_name(person));
        tokio::time::sleep(MUTATE_DELAY).await;
    }

    eprintln!("Done. {} contact(s) updated.", matching.len());
    Ok(())
}

pub async fn cmd_auto_contact_type(dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
    let user_groups: Vec<(&str, &str)> = all_groups.iter()
        .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
        .filter_map(|g| Some((g.name.as_deref()?, g.resource_name.as_deref()?)))
        .collect();

    let mut type_rns: Option<(String, String)> = None;
    let mut count = 0;

    for person in &contacts {
        let (has_person, has_company) = person_type_labels(person, &group_names);
        if has_person || has_company {
            continue;
        }

        let names = person.names.as_ref().and_then(|n| n.first());
        let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
        let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");
        let is_company = given.is_empty() && family.is_empty();
        let label = if is_company { TYPE_COMPANY_LABEL } else { TYPE_PERSON_LABEL };

        println!("{} -> {}", format_person_line(person, Some(&group_names)), label);
        count += 1;

        if dry_run {
            continue;
        }

        let resource_name = match person.resource_name.as_deref() {
            Some(rn) => rn,
            None => continue,
        };

        if type_rns.is_none() {
            type_rns = Some(ensure_type_labels_exist(&hub, &user_groups).await?);
        }
        let (ref person_rn, ref company_rn) = *type_rns.as_ref().unwrap();
        let target_rn = if is_company { company_rn } else { person_rn };

        let req = google_people1::api::ModifyContactGroupMembersRequest {
            resource_names_to_add: Some(vec![resource_name.to_string()]),
            resource_names_to_remove: None,
        };
        hub.contact_groups().members_modify(req, target_rn).doit().await?;
        tokio::time::sleep(MUTATE_DELAY).await;
    }

    if dry_run {
        eprintln!("(dry-run) Would auto-assign type labels to {} contact(s).", count);
    } else {
        eprintln!("Done. {} contact(s) updated.", count);
    }
    Ok(())
}

pub async fn cmd_check_contact_no_middle_name(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_no_middle_name(&hub, &contacts, &ctx).await?;
    Ok(())
}

async fn check_no_middle_name(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let has_middle = person.names.as_ref()
            .and_then(|n| n.first())
            .and_then(|n| n.middle_name.as_deref())
            .is_some_and(|v| !v.is_empty());
        if !has_middle {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

            if ctx.fix && !ctx.dry_run {
                use std::io::Write;
                loop {
                    eprint!("  [r]emove middle name / [e]dit contact / [s]kip? ");
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('r') => {
                            let resource_name = person.resource_name.as_deref()
                                .ok_or("Contact missing resource name")?;
                            let mut updated = person.clone();
                            if let Some(ref mut names) = updated.names
                                && let Some(first) = names.first_mut() {
                                    first.middle_name = None;
                                }
                            hub.people()
                                .update_contact(updated, resource_name)
                                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                                .doit()
                                .await?;
                            eprintln!("  Removed middle name.");
                            tokio::time::sleep(MUTATE_DELAY).await;
                            break;
                        }
                        Some('e') => {
                            interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                            break;
                        }
                        Some('s') => {
                            eprintln!("  Skipped.");
                            break;
                        }
                        _ => eprintln!("  Invalid choice. Enter r, e, or s."),
                    }
                }
            }
        }
        count += 1;
    }

    if !ctx.quiet && count > 0 && ctx.header.is_some() {
        println!();
    }

    Ok(count)
}

pub async fn cmd_check_contact_no_nickname(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;
    let group_names = build_group_name_map(&all_groups);
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
    let ctx = CheckContext { fix, dry_run, prefix: "", header: None, quiet: false, user_groups: &user_groups, label_names: &label_names, group_names: &group_names };
    check_no_nickname(&hub, &contacts, &ctx).await?;
    Ok(())
}

async fn check_no_nickname(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    ctx: &CheckContext<'_>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let has_nickname = person.nicknames.as_ref()
            .is_some_and(|nicks| nicks.iter().any(|n| n.value.as_ref().is_some_and(|v| !v.is_empty())));
        if !has_nickname {
            continue;
        }

        if !ctx.quiet {
            if count == 0
                && let Some(header) = ctx.header {
                    println!("=== {} ===", header);
                }
            println!("{}{}", ctx.prefix, format_person_line(person, Some(ctx.group_names)));

            if ctx.fix && !ctx.dry_run {
                use std::io::Write;
                loop {
                    eprint!("  [r]emove nickname / [e]dit contact / [s]kip? ");
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('r') => {
                            let resource_name = person.resource_name.as_deref()
                                .ok_or("Contact missing resource name")?;
                            let mut updated = person.clone();
                            updated.nicknames = Some(vec![]);
                            hub.people()
                                .update_contact(updated, resource_name)
                                .update_person_fields(FieldMask::new::<&str>(&["nicknames"]))
                                .doit()
                                .await?;
                            eprintln!("  Removed nickname.");
                            tokio::time::sleep(MUTATE_DELAY).await;
                            break;
                        }
                        Some('e') => {
                            interactive_edit_contact(hub, person, ctx.user_groups, ctx.label_names, ctx.group_names).await?;
                            break;
                        }
                        Some('s') => {
                            eprintln!("  Skipped.");
                            break;
                        }
                        _ => eprintln!("  Invalid choice. Enter r, e, or s."),
                    }
                }
            }
        }
        count += 1;
    }

    if !ctx.quiet && count > 0 && ctx.header.is_some() {
        println!();
    }

    Ok(count)
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

pub async fn cmd_edit_contact(search: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    let group_names = build_group_name_map(&all_groups);
    let user_groups_owned: Vec<(String, String)> = all_groups.iter()
        .filter(|g| g.group_type.as_deref() == Some("USER_CONTACT_GROUP"))
        .filter_map(|g| {
            let name = g.name.as_deref()?;
            let rn = g.resource_name.as_deref()?;
            Some((name.to_string(), rn.to_string()))
        })
        .collect();
    let label_names: Vec<String> = user_groups_owned.iter().map(|(name, _)| name.clone()).collect();
    let user_groups: Vec<(&str, &str)> = user_groups_owned.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();

    let person = if matches.len() == 1 {
        matches[0]
    } else {
        println!("Multiple contacts found:");
        for (i, p) in matches.iter().enumerate() {
            println!("  {}: {}", i + 1, format_person_line(p, None));
        }
        use std::io::Write;
        eprint!("Pick a contact [1-{}]: ", matches.len());
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let idx: usize = input.trim().parse().unwrap_or(0);
        if idx < 1 || idx > matches.len() {
            eprintln!("Invalid selection.");
            return Ok(());
        }
        matches[idx - 1]
    };

    interactive_edit_contact(&hub, person, &user_groups, &label_names, &group_names).await?;
    Ok(())
}

fn print_person_details(person: &google_people1::api::Person, group_names: Option<&std::collections::HashMap<String, String>>) {
    let names = person.names.as_ref().and_then(|n| n.first());
    let given = names.and_then(|n| n.given_name.as_deref()).unwrap_or("");
    let family = names.and_then(|n| n.family_name.as_deref()).unwrap_or("");
    let suffix = names.and_then(|n| n.honorific_suffix.as_deref()).unwrap_or("");
    let prefix_name = names.and_then(|n| n.honorific_prefix.as_deref()).unwrap_or("");
    let middle = names.and_then(|n| n.middle_name.as_deref()).unwrap_or("");

    println!("Given name: {}", given);
    println!("Family name: {}", family);
    println!("Suffix: {}", suffix);
    if !middle.is_empty() { println!("Middle name: {}", middle); }
    if !prefix_name.is_empty() { println!("Prefix: {}", prefix_name); }

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

    // Organization — always show company field
    let org = person.organizations.as_ref().and_then(|o| o.first());
    let company = org.and_then(|o| o.name.as_deref()).unwrap_or("");
    println!("Company: {}", company);
    if let Some(orgs) = &person.organizations {
        for o in orgs {
            let title = o.title.as_deref().unwrap_or("");
            let dept = o.department.as_deref().unwrap_or("");
            if !title.is_empty() { println!("  Title: {}", title); }
            if !dept.is_empty() { println!("  Department: {}", dept); }
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

    let labels: Vec<String> = person.memberships.as_ref().map(|memberships| {
        memberships.iter().filter_map(|m| {
            let rn = m.contact_group_membership.as_ref()?.contact_group_resource_name.as_deref()?;
            if rn == "contactGroups/myContacts" { return None; }
            let display = group_names
                .and_then(|gn| gn.get(rn))
                .map(|s| s.as_str())
                .unwrap_or(rn);
            Some(display.to_string())
        }).collect()
    }).unwrap_or_default();
    println!("Labels: {}", if labels.is_empty() { "(none)".to_string() } else { labels.join(", ") });

    if let Some(rn) = &person.resource_name {
        println!("Resource: {}", rn);
    }
}

pub async fn cmd_check_contact_label_regexp(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config();
    let allow = match config.check_contact_label_regexp.allow {
        Some(ref pattern) => pattern.clone(),
        None => {
            eprintln!("No [check-contact-label-regexp] allow regex configured in config.toml.");
            eprintln!("Add a section like:");
            eprintln!("  [check-contact-label-regexp]");
            eprintln!("  allow = '^[A-Z][a-z]+$'");
            return Ok(());
        }
    };
    let re = regex::Regex::new(&allow)?;

    let hub = build_hub().await?;
    let all_groups = fetch_all_contact_groups(&hub).await?;

    let bad_labels: Vec<&google_people1::api::ContactGroup> = all_groups.iter().filter(|g| {
        g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
            && g.name.as_deref().is_some_and(|n| !re.is_match(n))
    }).collect();

    for group in &bad_labels {
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "emailAddresses", "phoneNumbers", "nicknames", "memberships"]).await?;
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
    let contacts = fetch_all_contacts(&hub, &["names", "organizations", "phoneNumbers", "emailAddresses", "nicknames", "memberships"]).await?;
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
                println!("{}", format_person_line(person, None));
                count += 1;
                if fix {
                    if dry_run {
                        eprintln!("  (dry-run) would prompt for action");
                        continue;
                    }
                    let phone = pn.value.as_deref().unwrap_or("");
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
    let all_contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "phoneNumbers", "memberships", "organizations", "nicknames"]).await?;
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
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Phones missing country code (check-phone-countrycode)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let no_country = check_phone_issues(
            &hub, &all_contacts,
            |v| is_fixable_phone(v) && !has_country_code(v),
            move |v| add_country_code(v, &country_owned),
            &ctx,
        ).await?;
        results.push(("check-phone-countrycode", no_country));
    }

    if !skip.contains("check-phone-format") {
        log("check-phone-format");
        let country_owned2 = country.to_string();
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Phones not in +CC-NUMBER format (check-phone-format)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let bad_format = check_phone_issues(
            &hub, &all_contacts,
            |v| is_fixable_phone(v) && !is_correct_phone_format(v),
            move |v| fix_phone_format(v, &country_owned2),
            &ctx,
        ).await?;
        results.push(("check-phone-format", bad_format));
    }

    if !skip.contains("check-contact-given-name-regexp") {
        log("check-contact-given-name-regexp");
        if config.check_contact_given_name_regexp.allow.is_some() {
            let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Given name doesn't match allow regex (check-contact-given-name-regexp)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
            let given_name_regexp = check_given_name_regexp(&hub, &all_contacts, &config.check_contact_given_name_regexp, &ctx).await?;
            results.push(("check-contact-given-name-regexp", given_name_regexp));
        } else {
            eprintln!("Warning: check-contact-given-name-regexp has no allow regex configured, skipping.");
        }
    }

    if !skip.contains("check-contact-family-name-regexp") {
        log("check-contact-family-name-regexp");
        if config.check_contact_family_name_regexp.allow.is_some() {
            let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Family name doesn't match allow regex (check-contact-family-name-regexp)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
            let family_name_regexp = check_family_name_regexp(&hub, &all_contacts, &config.check_contact_family_name_regexp, &ctx).await?;
            results.push(("check-contact-family-name-regexp", family_name_regexp));
        } else {
            eprintln!("Warning: check-contact-family-name-regexp has no allow regex configured, skipping.");
        }
    }

    if !skip.contains("check-contact-suffix-regexp") {
        log("check-contact-suffix-regexp");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Suffix doesn't match allow regex (check-contact-suffix-regexp)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let suffix_regexp = check_suffix_regexp(&hub, &all_contacts, &config.check_contact_suffix_regexp, &ctx).await?;
        results.push(("check-contact-suffix-regexp", suffix_regexp));
    }

    if !skip.contains("check-contact-no-given-name") {
        log("check-contact-no-given-name");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contacts with family name but no given name (check-contact-no-given-name)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let no_given = check_no_given_name(&hub, &all_contacts, &ctx).await?;
        results.push(("check-contact-no-given-name", no_given));
    }

    if !skip.contains("check-contact-no-identity") {
        log("check-contact-no-identity");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contacts with no given name and no company (check-contact-no-identity)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let no_identity = check_no_identity(&hub, &all_contacts, &ctx).await?;
        results.push(("check-contact-no-identity", no_identity));
    }

    {
        log("check-contact-given-name-known");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Given name not in allowed list (check-contact-given-name-known)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let given_name_known = check_given_name_known(&hub, &all_contacts, &config.check_contact_given_name_known.names, &ctx).await?;
        results.push(("check-contact-given-name-known", given_name_known));
    }

    if !skip.contains("check-contact-name-is-company") {
        log("check-contact-name-is-company");
        if !config.check_contact_name_is_company.companies.is_empty() {
            let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contact name matches company name (check-contact-name-is-company)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
            let name_company = check_name_is_company(&hub, &all_contacts, &config.check_contact_name_is_company.companies, &ctx).await?;
            results.push(("check-contact-name-is-company", name_company));
        } else {
            eprintln!("Warning: check-contact-name-is-company has no companies configured, skipping.");
        }
    }

    if !skip.contains("check-contact-company-known") {
        log("check-contact-company-known");
        if !config.check_contact_name_is_company.companies.is_empty() {
            let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Company not in configured list (check-contact-company-known)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
            let company_known = check_company_known(&hub, &all_contacts, &config.check_contact_name_is_company.companies, &ctx).await?;
            results.push(("check-contact-company-known", company_known));
        } else {
            eprintln!("Warning: check-contact-company-known has no companies configured, skipping.");
        }
    }

    if !skip.contains("check-contact-displayname-duplicate") {
        log("check-contact-displayname-duplicate");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Duplicate contact names (check-contact-displayname-duplicate)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let name_dup = check_name_duplicate(&hub, &all_contacts, &ctx).await?;
        results.push(("check-contact-displayname-duplicate", name_dup));
    }

    if !skip.contains("check-contact-type") {
        log("check-contact-type");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contacts missing or having both TypePerson/TypeCompany (check-contact-type)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let type_count = check_contact_type(&hub, &all_contacts, &ctx).await?;
        results.push(("check-contact-type", type_count));
    }

    if !skip.contains("check-contact-no-middle-name") {
        log("check-contact-no-middle-name");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contacts with middle name (check-contact-no-middle-name)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let middle_name_count = check_no_middle_name(&hub, &all_contacts, &ctx).await?;
        results.push(("check-contact-no-middle-name", middle_name_count));
    }

    if !skip.contains("check-contact-no-nickname") {
        log("check-contact-no-nickname");
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contacts with nickname (check-contact-no-nickname)"), quiet: stats, user_groups: &user_groups_regexp, label_names: &label_names_regexp, group_names: &group_names_for_regexp };
        let nickname_count = check_no_nickname(&hub, &all_contacts, &ctx).await?;
        results.push(("check-contact-no-nickname", nickname_count));
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
        let ctx = CheckContext { fix, dry_run, prefix, header: hdr("Contacts without label (check-contact-no-label)"), quiet: stats, user_groups: &user_groups, label_names: &label_names, group_names: &group_names_for_regexp };
        let no_label = check_no_label(&hub, &all_contacts, &ctx).await?;
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

    if !skip.contains("check-contact-label-regexp") {
        log("check-contact-label-regexp");
        let label_regexp_count = if let Some(ref pattern) = config.check_contact_label_regexp.allow {
            let re = regex::Regex::new(pattern)?;
            let bad_labels: Vec<_> = all_groups.iter().filter(|g| {
                g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
                    && g.name.as_deref().is_some_and(|n| !re.is_match(n))
            }).collect();
            if !stats && !bad_labels.is_empty() {
                println!("=== Labels not matching regex (check-contact-label-regexp) ({}) ===", bad_labels.len());
                for group in &bad_labels {
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
            bad_labels.len()
        } else {
            eprintln!("Warning: check-contact-label-regexp has no allow regex configured, skipping.");
            0
        };
        results.push(("check-contact-label-regexp", label_regexp_count));
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

pub async fn cmd_compact_suffixes_for_contacts(dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "organizations"]).await?;

    // Group contacts by base name (given + family, no suffix)
    let mut base_groups: std::collections::HashMap<String, Vec<&google_people1::api::Person>> =
        std::collections::HashMap::new();
    for person in &contacts {
        let base = person_base_name(person);
        let key = if base.is_empty() { person_name(person) } else { base };
        if !key.is_empty() {
            base_groups.entry(key).or_default().push(person);
        }
    }

    let mut sorted_groups: Vec<(&str, &Vec<&google_people1::api::Person>)> = base_groups
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(name, group)| (name.as_str(), group))
        .collect();
    sorted_groups.sort_by_key(|(name, _)| *name);

    let mut total_changes = 0;

    for (base_name, group) in &sorted_groups {
        // Collect current suffixes
        let suffix_contacts: Vec<(&google_people1::api::Person, Option<u32>)> = group
            .iter()
            .map(|p| {
                let suffix = p.names.as_ref()
                    .and_then(|names| names.first())
                    .and_then(|n| n.honorific_suffix.as_deref())
                    .and_then(|s| s.parse::<u32>().ok());
                (*p, suffix)
            })
            .collect();

        // Target: suffixes 1..=N for N contacts
        let n = suffix_contacts.len() as u32;
        let target: std::collections::BTreeSet<u32> = (1..=n).collect();

        // Current valid suffixes (>= 1, <= N, no duplicates counted)
        let mut suffix_counts: std::collections::HashMap<Option<u32>, usize> = std::collections::HashMap::new();
        for (_, s) in &suffix_contacts {
            *suffix_counts.entry(*s).or_default() += 1;
        }
        let has_duplicates = suffix_counts.values().any(|c| *c > 1);

        let current: std::collections::BTreeSet<u32> = suffix_contacts
            .iter()
            .filter_map(|(_, s)| *s)
            .filter(|s| *s >= 1 && *s <= n)
            .collect();

        if current == target && !has_duplicates {
            continue; // Already compact
        }

        // Holes: target suffixes not in current set
        let holes: Vec<u32> = target.difference(&current).copied().collect();

        // Contacts that need a new suffix: no suffix, suffix=0, suffix>N, or duplicate
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        let mut needs_reassign: Vec<usize> = Vec::new();
        for (i, (_, s)) in suffix_contacts.iter().enumerate() {
            match s {
                Some(v) if *v >= 1 && *v <= n => {
                    if !seen.insert(*v) {
                        needs_reassign.push(i);
                    }
                }
                _ => {
                    needs_reassign.push(i);
                }
            }
        }

        if needs_reassign.is_empty() {
            continue;
        }

        println!("\"{}\" ({} contacts):", base_name, group.len());
        for (person, _suffix) in &suffix_contacts {
            println!("  - {}", format_person_line(person, None));
        }

        // Sort needs_reassign by current suffix descending (high suffixes folded into holes first)
        needs_reassign.sort_by(|a, b| {
            let sa = suffix_contacts[*a].1.unwrap_or(0);
            let sb = suffix_contacts[*b].1.unwrap_or(0);
            sb.cmp(&sa)
        });

        for (hole, idx) in holes.iter().zip(needs_reassign.iter()) {
            let (person, old_suffix) = &suffix_contacts[*idx];
            let old_str = match old_suffix {
                Some(s) => s.to_string(),
                None => "none".to_string(),
            };
            println!("  {} -> suffix \"{}\" (was \"{}\")", format_person_line(person, None), hole, old_str);

            if !dry_run {
                let resource_name = person
                    .resource_name
                    .as_deref()
                    .ok_or("Contact missing resource name")?;
                let mut updated = (*person).clone();
                match updated.names {
                    Some(ref mut names) if !names.is_empty() => {
                        names[0].honorific_suffix = Some(hole.to_string());
                    }
                    _ => {
                        updated.names = Some(vec![google_people1::api::Name {
                            honorific_suffix: Some(hole.to_string()),
                            ..Default::default()
                        }]);
                    }
                }
                hub.people()
                    .update_contact(updated, resource_name)
                    .update_person_fields(FieldMask::new::<&str>(&["names"]))
                    .doit()
                    .await?;
                tokio::time::sleep(MUTATE_DELAY).await;
            }
            total_changes += 1;
        }
        println!();
    }

    if total_changes == 0 {
        println!("All suffixes are already compact.");
    } else if dry_run {
        println!("{} change(s) would be made.", total_changes);
    } else {
        println!("{} suffix(es) updated.", total_changes);
    }

    Ok(())
}
