use google_people1::FieldMask;

use crate::helpers::*;

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
    let mut fields = vec!["names", "phoneNumbers"];
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

        let mut parts = vec![name.to_string()];

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

async fn check_name_issues<F>(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    predicate: F,
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>>
where
    F: Fn(&str) -> bool,
{
    let filtered: Vec<&google_people1::api::Person> = contacts.iter().filter(|p| {
        let name = person_name(p);
        !name.is_empty() && predicate(name)
    }).collect();

    if !quiet {
        if !filtered.is_empty() {
            if let Some(header) = header {
                println!("=== {} ({}) ===", header, filtered.len());
            }
        }

        for person in &filtered {
            let name = person_name(person);
            let email = person_email(person);
            print_name_with_email(name, email, prefix);

            if fix && !dry_run {
                interactive_name_fix(hub, person, name).await?;
            }
        }

        if !filtered.is_empty() && header.is_some() {
            println!();
        }
    }

    Ok(filtered.len())
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
                            print_phone_fix(name, val, &fixed, fix, dry_run, prefix);
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

pub async fn cmd_check_contact_name_english(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_issues(&hub, &contacts, |name| !is_english_name(name), fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_name_caps(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_issues(&hub, &contacts, |name| is_all_caps(name), fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_name_first_capital_letter(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_issues(&hub, &contacts, |name| !starts_with_capital(name), fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_name_firstname_numeric(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names"]).await?;
    check_name_firstname_numeric(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_name_firstname_numeric(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let name_entry = match person.names.as_ref().and_then(|n| n.first()) {
            Some(n) => n,
            None => continue,
        };
        let given = name_entry.given_name.as_deref().unwrap_or("");
        if !is_numeric_string(given) {
            continue;
        }

        let family = name_entry.family_name.as_deref().unwrap_or("");
        let display = person_display_name(person);

        if !quiet {
            if count == 0 {
                if let Some(header) = header {
                    println!("=== {} ===", header);
                }
            }
            if !family.is_empty() {
                if fix || dry_run {
                    println!("{}{} -> given: \"{}\", suffix: \"{}\"", prefix, display, family, given);
                } else {
                    println!("{}{} (given: \"{}\", family: \"{}\")", prefix, display, given, family);
                }
            } else {
                println!("{}{} (given: \"{}\")", prefix, display, given);
            }

            if fix && !dry_run {
                if !family.is_empty() {
                    // Move family to given, move numeric given to suffix
                    let resource_name = person
                        .resource_name
                        .as_deref()
                        .ok_or("Contact missing resource name")?;
                    let mut updated = person.clone();
                    if let Some(ref mut names) = updated.names {
                        if let Some(first) = names.first_mut() {
                            first.given_name = Some(family.to_string());
                            first.family_name = None;
                            first.honorific_suffix = Some(given.to_string());
                            first.unstructured_name = Some(format!("{} {}", family, given));
                        }
                    }
                    hub.people()
                        .update_contact(updated, resource_name)
                        .update_person_fields(FieldMask::new::<&str>(&["names"]))
                        .doit()
                        .await?;
                    eprintln!("{}  Fixed: given=\"{}\", suffix=\"{}\"", prefix, family, given);
                    tokio::time::sleep(MUTATE_DELAY).await;
                } else {
                    eprintln!("{}  No family name to swap — skipped.", prefix);
                }
            }
        }
        count += 1;
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_name_firstname_space(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "organizations"]).await?;
    check_name_firstname_space(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_name_firstname_space(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let given = person.names.as_ref()
            .and_then(|names| names.first())
            .and_then(|n| n.given_name.as_deref())
            .unwrap_or("");
        if !given.contains(' ') {
            continue;
        }

        if !quiet {
            if count == 0 {
                if let Some(header) = header {
                    println!("=== {} ===", header);
                }
            }
            let display = person_display_name(person);
            let email = person_email(person);
            println!("{}{} (given: \"{}\")", prefix, display, given);
            if !email.is_empty() {
                println!("{}  email: {}", prefix, email);
            }

            if fix && !dry_run {
                let parts: Vec<&str> = given.splitn(2, ' ').collect();
                let has_two_parts = parts.len() == 2 && !parts[1].is_empty();

                use std::io::Write;
                loop {
                    if has_two_parts {
                        eprint!("  [p]split \"{}\"/\"{}\" / [c]ompany / [r]ename / [d]elete / [s]kip? ", parts[0], parts[1]);
                    } else {
                        eprint!("  [c]ompany / [r]ename / [d]elete / [s]kip? ");
                    }
                    std::io::stderr().flush()?;
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    match input.trim().chars().next() {
                        Some('p') if has_two_parts => {
                            let resource_name = person
                                .resource_name
                                .as_deref()
                                .ok_or("Contact missing resource name")?;
                            let mut updated = person.clone();
                            if let Some(ref mut names) = updated.names {
                                if let Some(first) = names.first_mut() {
                                    first.given_name = Some(parts[0].to_string());
                                    first.family_name = Some(parts[1].to_string());
                                    first.unstructured_name = Some(given.to_string());
                                }
                            }
                            hub.people()
                                .update_contact(updated, resource_name)
                                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                                .doit()
                                .await?;
                            eprintln!("  Split to given: \"{}\", family: \"{}\"", parts[0], parts[1]);
                            tokio::time::sleep(MUTATE_DELAY).await;
                            break;
                        }
                        Some('c') => {
                            let resource_name = person
                                .resource_name
                                .as_deref()
                                .ok_or("Contact missing resource name")?;
                            let mut updated = person.clone();
                            // Move the full given name to organization
                            let org = google_people1::api::Organization {
                                name: Some(given.to_string()),
                                ..Default::default()
                            };
                            updated.organizations = Some(vec![org]);
                            // Clear the name
                            if let Some(ref mut names) = updated.names {
                                if let Some(first) = names.first_mut() {
                                    first.given_name = None;
                                    first.family_name = None;
                                    first.unstructured_name = None;
                                }
                            }
                            hub.people()
                                .update_contact(updated, resource_name)
                                .update_person_fields(FieldMask::new::<&str>(&["names", "organizations"]))
                                .doit()
                                .await?;
                            eprintln!("  Moved \"{}\" to company name.", given);
                            tokio::time::sleep(MUTATE_DELAY).await;
                            break;
                        }
                        Some('r') => {
                            let new_name = prompt_new_name(display)?;
                            let resource_name = person
                                .resource_name
                                .as_deref()
                                .ok_or("Contact missing resource name")?;
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
                            break;
                        }
                        Some('d') => {
                            let resource_name = person
                                .resource_name
                                .as_deref()
                                .ok_or("Contact missing resource name")?;
                            hub.people().delete_contact(resource_name).doit().await?;
                            eprintln!("  Deleted.");
                            tokio::time::sleep(MUTATE_DELAY).await;
                            break;
                        }
                        Some('s') => {
                            eprintln!("  Skipped.");
                            break;
                        }
                        _ => eprintln!("  Invalid choice."),
                    }
                }
            }
        }
        count += 1;
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

async fn check_name_order(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if has_reversed_name(person) {
            if !quiet {
                if count == 0 {
                    if let Some(header) = header {
                        println!("=== {} ===", header);
                    }
                }
                let name = person_display_name(person);
                let fixed = compute_fixed_name(name);
                println!("{}{} -> {}", prefix, name, fixed);
            }
            count += 1;

            if fix && !dry_run && !quiet {
                use std::io::Write;
                std::io::stdout().flush()?;
                let name = person_display_name(person);
                let fixed = compute_fixed_name(name);
                let resource_name = person
                    .resource_name
                    .as_deref()
                    .ok_or("Contact missing resource name")?;

                match prompt_fix_action(name)? {
                    'r' => {
                        let new_name = prompt_new_name_with_default(name, &fixed)?;
                        let mut updated = person.clone();
                        if let Some(ref mut names) = updated.names {
                            if let Some(first) = names.first_mut() {
                                if let Some((given, family)) = new_name.split_once(' ') {
                                    first.given_name = Some(given.to_string());
                                    first.family_name = Some(family.to_string());
                                } else {
                                    first.given_name = Some(new_name.clone());
                                    first.family_name = None;
                                }
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
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_name_order(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_order(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_displayname_duplicate(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "phoneNumbers"]).await?;
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
                    interactive_name_fix(hub, person, display).await?;
                }
            }
        }

        if header.is_some() {
            println!();
        }
    }

    Ok(count)
}

pub async fn cmd_check_contact_samename_suffix(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names"]).await?;
    check_samename_suffix(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_samename_suffix(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    // Group contacts by base name (display name with numeric suffix stripped)
    let mut base_groups: std::collections::HashMap<String, Vec<(&google_people1::api::Person, Option<u32>)>> =
        std::collections::HashMap::new();
    for person in contacts {
        let name = person_name(person);
        if name.is_empty() {
            continue;
        }
        let (base, suffix) = split_name_suffix(name);
        base_groups.entry(base.to_string()).or_default().push((person, suffix));
    }

    let mut count = 0;
    let mut issues: Vec<(String, Vec<String>, Vec<(&google_people1::api::Person, Option<u32>)>)> = Vec::new();

    for (base, group) in &base_groups {
        if group.len() < 2 {
            // Only one contact with this base name — but if it has a suffix, that's odd
            if group.len() == 1 && group[0].1.is_some() {
                let display = person_display_name(group[0].0);
                issues.push((base.clone(), vec![
                    format!("{} has suffix but is the only contact with base name \"{}\"", display, base),
                ], group.clone()));
                count += 1;
            }
            continue;
        }

        // Multiple contacts share the same base name — check suffixes
        let mut problems = Vec::new();
        let suffixes: Vec<Option<u32>> = group.iter().map(|(_, s)| *s).collect();

        // Check: all must have a suffix
        let missing: Vec<_> = group.iter()
            .filter(|(_, s)| s.is_none())
            .collect();
        for (person, _) in &missing {
            let display = person_display_name(person);
            problems.push(format!("{} is missing a numeric suffix", display));
            count += 1;
        }

        // Check: suffixes should be 1..=N
        let mut nums: Vec<u32> = suffixes.iter().filter_map(|s| *s).collect();
        nums.sort();
        let expected: Vec<u32> = (1..=group.len() as u32).collect();
        if !missing.is_empty() || nums != expected {
            if missing.is_empty() {
                // All have suffixes but they're not sequential
                let actual: Vec<String> = nums.iter().map(|n| n.to_string()).collect();
                let expected_str: Vec<String> = expected.iter().map(|n| n.to_string()).collect();
                problems.push(format!(
                    "suffixes are [{}], expected [{}]",
                    actual.join(", "),
                    expected_str.join(", "),
                ));
                count += 1;
            }
        }

        if !problems.is_empty() {
            issues.push((base.clone(), problems, group.clone()));
        }
    }

    issues.sort_by(|(a, _, _), (b, _, _)| a.cmp(b));

    if !quiet && !issues.is_empty() {
        if let Some(header) = header {
            println!("=== {} ({}) ===", header, count);
        }
        for (base, problems, group) in &issues {
            println!("{}\"{}\":", prefix, base);
            for problem in problems {
                println!("{}  - {}", prefix, problem);
            }

            if fix || dry_run {
                if group.len() == 1 {
                    // Lone contact with a suffix — remove it
                    let (person, _) = &group[0];
                    let old_display = person_display_name(person);
                    println!("{}  {} -> {}", prefix, old_display, base);

                    if fix && !dry_run {
                        let resource_name = person
                            .resource_name
                            .as_deref()
                            .ok_or("Contact missing resource name")?;
                        let mut updated = (*person).clone();
                        if let Some(ref mut names) = updated.names {
                            if let Some(first) = names.first_mut() {
                                first.honorific_suffix = None;
                                first.family_name = None;
                                first.unstructured_name = Some(base.clone());
                            }
                        }
                        hub.people()
                            .update_contact(updated, resource_name)
                            .update_person_fields(FieldMask::new::<&str>(&["names"]))
                            .doit()
                            .await?;
                        eprintln!("{}  Updated.", prefix);
                        tokio::time::sleep(MUTATE_DELAY).await;
                    }
                } else {
                    // Multiple contacts — renumber sequentially.
                    // First fix contacts missing a suffix (assign lowest available),
                    // then renumber all to be sequential 1..N.

                    // Sort group by existing suffix (None first, then by number)
                    let mut sorted_group = group.clone();
                    sorted_group.sort_by_key(|(_, s)| s.unwrap_or(0));

                    for (i, (person, old_suffix)) in sorted_group.iter().enumerate() {
                        let new_num = (i + 1) as u32;
                        if *old_suffix == Some(new_num) {
                            continue; // Already correct
                        }
                        let old_display = person_display_name(person);
                        let new_display = format!("{} {}", base, new_num);
                        println!("{}  {} -> {}", prefix, old_display, new_display);

                        if fix && !dry_run {
                            let resource_name = person
                                .resource_name
                                .as_deref()
                                .ok_or("Contact missing resource name")?;
                            let mut updated = (*person).clone();
                            if let Some(ref mut names) = updated.names {
                                if let Some(first) = names.first_mut() {
                                    first.honorific_suffix = Some(format!("{}", new_num));
                                    first.family_name = None;
                                    first.unstructured_name = Some(new_display);
                                }
                            }
                            hub.people()
                                .update_contact(updated, resource_name)
                                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                                .doit()
                                .await?;
                            eprintln!("{}  Updated.", prefix);
                            tokio::time::sleep(MUTATE_DELAY).await;
                        }
                    }
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
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
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
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
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

fn check_invalid_emails(contacts: &[google_people1::api::Person], prefix: &str, header: Option<&str>, quiet: bool) -> usize {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            for email in emails {
                if let Some(val) = email.value.as_deref() {
                    if !is_valid_email(val) {
                        if !quiet {
                            if count == 0 {
                                if let Some(header) = header {
                                    println!("=== {} ===", header);
                                }
                            }
                            let name = person_display_name(person);
                            println!("{}{} | {}", prefix, name, val);
                        }
                        count += 1;
                    }
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    count
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
                print_name_with_email(name, email, prefix);
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
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
    check_duplicate_phones(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_contact_email() -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_invalid_emails(&contacts, "", None, false);
    Ok(())
}

async fn check_email_caps(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        if let Some(emails) = &person.email_addresses {
            let has_caps = emails.iter().any(|e| {
                e.value.as_deref().is_some_and(|v| v != v.to_lowercase().as_str())
            });
            if has_caps {
                if !quiet {
                    if count == 0 {
                        if let Some(header) = header {
                            println!("=== {} ===", header);
                        }
                    }
                    let name = person_display_name(person);
                    for email in emails {
                        if let Some(val) = email.value.as_deref() {
                            if val != val.to_lowercase().as_str() {
                                println!("{}{} | {} -> {}", prefix, name, val, val.to_lowercase());
                            }
                        }
                    }
                }
                count += emails.iter().filter(|e| {
                    e.value.as_deref().is_some_and(|v| v != v.to_lowercase().as_str())
                }).count();

                if fix && !dry_run && !quiet {
                    use std::io::Write;
                    std::io::stdout().flush()?;
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
                    eprintln!("  Fixed emails for {}", name);
                    tokio::time::sleep(MUTATE_DELAY).await;
                }
            }
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
}

pub async fn cmd_check_contact_email_caps(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_email_caps(&hub, &contacts, fix, dry_run, "", None, false).await?;
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
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_duplicate_emails(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_phone_no_label(hub: &HubType, contacts: &[google_people1::api::Person], fix: bool, dry_run: bool, prefix: &str, header: Option<&str>, quiet: bool) -> Result<usize, Box<dyn std::error::Error>> {
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
                    if let Some(new_label) = prompt_phone_label_fix(name)? {
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

pub async fn cmd_check_phone_no_label(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
    check_phone_no_label(&hub, &contacts, fix, dry_run, "", None, false).await?;
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
                    let new_label = prompt_phone_label_fix(name)?;
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
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
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
        fetch_all_contacts(&hub, &["names", "emailAddresses", "memberships"]).await?
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

pub async fn cmd_check_contact_name_numeric_surname(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names"]).await?;
    check_name_numeric_surname(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

async fn check_name_numeric_surname(
    hub: &HubType,
    contacts: &[google_people1::api::Person],
    fix: bool,
    dry_run: bool,
    prefix: &str,
    header: Option<&str>,
    quiet: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0;
    for person in contacts {
        let names = match &person.names {
            Some(names) => names,
            None => continue,
        };
        let name = match names.first() {
            Some(n) => n,
            None => continue,
        };
        let family = match name.family_name.as_deref() {
            Some(f) => f,
            None => continue,
        };
        if !is_numeric_string(family) {
            continue;
        }

        if !quiet {
            if count == 0 {
                if let Some(header) = header {
                    println!("=== {} ===", header);
                }
            }
            let display = person_display_name(person);
            let given = name.given_name.as_deref().unwrap_or("");
            if fix || dry_run {
                println!("{}{} -> given: \"{}\", suffix: \"{}\"", prefix, display, given, family);
            } else {
                println!("{}{} (surname: \"{}\")", prefix, display, family);
            }
        }
        count += 1;

        if fix && !dry_run && !quiet {
            let given = name.given_name.as_deref().unwrap_or("");
            let resource_name = person
                .resource_name
                .as_deref()
                .ok_or("Contact missing resource name")?;
            let mut updated = person.clone();
            if let Some(ref mut names) = updated.names {
                if let Some(first) = names.first_mut() {
                    first.honorific_suffix = Some(family.to_string());
                    first.family_name = None;
                    first.unstructured_name = Some(format!("{} {}", given, family));
                }
            }
            hub.people()
                .update_contact(updated, resource_name)
                .update_person_fields(FieldMask::new::<&str>(&["names"]))
                .doit()
                .await?;
            eprintln!("{}  Updated.", prefix);
            tokio::time::sleep(MUTATE_DELAY).await;
        }
    }
    if !quiet && count > 0 && header.is_some() { println!(); }
    Ok(count)
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
    let contacts = fetch_all_contacts(&hub, &["names", "memberships"]).await?;
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
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
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
                                if let Some(new_label) = prompt_phone_label_fix(name)? {
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

    let mut results: Vec<(&str, usize)> = Vec::new();

    let prefix = if stats { "" } else { "  " };
    let hdr = |s: &'static str| -> Option<&'static str> { if stats { None } else { Some(s) } };
    let log = |name: &str| { if verbose { eprintln!("Running {}...", name); } };

    if !skip.contains("check-contact-name-english") {
        log("check-contact-name-english");
        let non_english = check_name_issues(
            &hub, &all_contacts, |name| !is_english_name(name),
            fix, dry_run, prefix, hdr("Non-English names (check-contact-name-english)"), stats,
        ).await?;
        results.push(("check-contact-name-english", non_english));
    }

    if !skip.contains("check-contact-name-caps") {
        log("check-contact-name-caps");
        let all_caps = check_name_issues(
            &hub, &all_contacts, |name| is_all_caps(name),
            fix, dry_run, prefix, hdr("All-caps names (check-contact-name-caps)"), stats,
        ).await?;
        results.push(("check-contact-name-caps", all_caps));
    }

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

    if !skip.contains("check-contact-name-first-capital-letter") {
        log("check-contact-name-first-capital-letter");
        let first_cap = check_name_issues(
            &hub, &all_contacts, |name| !starts_with_capital(name),
            fix, dry_run, prefix, hdr("Names not starting with capital letter (check-contact-name-first-capital-letter)"), stats,
        ).await?;
        results.push(("check-contact-name-first-capital-letter", first_cap));
    }

    if !skip.contains("check-contact-name-firstname-numeric") {
        log("check-contact-name-firstname-numeric");
        let firstname_numeric = check_name_firstname_numeric(&hub, &all_contacts, fix, dry_run, prefix, hdr("Numeric first name (check-contact-name-firstname-numeric)"), stats).await?;
        results.push(("check-contact-name-firstname-numeric", firstname_numeric));
    }

    if !skip.contains("check-contact-name-firstname-space") {
        log("check-contact-name-firstname-space");
        let firstname_space = check_name_firstname_space(&hub, &all_contacts, fix, dry_run, prefix, hdr("First name contains space (check-contact-name-firstname-space)"), stats).await?;
        results.push(("check-contact-name-firstname-space", firstname_space));
    }

    if !skip.contains("check-contact-name-order") {
        log("check-contact-name-order");
        let name_order = check_name_order(&hub, &all_contacts, fix, dry_run, prefix, hdr("Reversed name order (check-contact-name-order)"), stats).await?;
        results.push(("check-contact-name-order", name_order));
    }

    if !skip.contains("check-contact-displayname-duplicate") {
        log("check-contact-displayname-duplicate");
        let name_dup = check_name_duplicate(&hub, &all_contacts, fix, dry_run, prefix, hdr("Duplicate contact names (check-contact-displayname-duplicate)"), stats).await?;
        results.push(("check-contact-displayname-duplicate", name_dup));
    }

    if !skip.contains("check-contact-name-numeric-surname") {
        log("check-contact-name-numeric-surname");
        let numeric_surname = check_name_numeric_surname(&hub, &all_contacts, fix, dry_run, prefix, hdr("Numeric surnames (check-contact-name-numeric-surname)"), stats).await?;
        results.push(("check-contact-name-numeric-surname", numeric_surname));
    }

    if !skip.contains("check-contact-samename-suffix") {
        log("check-contact-samename-suffix");
        let samename_suffix = check_samename_suffix(&hub, &all_contacts, fix, dry_run, prefix, hdr("Same-name contacts with bad suffixes (check-contact-samename-suffix)"), stats).await?;
        results.push(("check-contact-samename-suffix", samename_suffix));
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

    if !skip.contains("check-phone-no-label") {
        log("check-phone-no-label");
        let phone_no_label = check_phone_no_label(&hub, &all_contacts, fix, dry_run, prefix, hdr("Phones without label (check-phone-no-label)"), stats).await?;
        results.push(("check-phone-no-label", phone_no_label));
    }

    if !skip.contains("check-phone-label-english") {
        log("check-phone-label-english");
        let phone_label_eng = check_phone_label_english(&hub, &all_contacts, fix, dry_run, prefix, hdr("Non-English phone labels (check-phone-label-english)"), stats).await?;
        results.push(("check-phone-label-english", phone_label_eng));
    }

    if !skip.contains("check-contact-email") {
        log("check-contact-email");
        results.push(("check-contact-email", check_invalid_emails(&all_contacts, prefix, hdr("Invalid emails (check-contact-email)"), stats)));
    }

    if !skip.contains("check-contact-email-caps") {
        log("check-contact-email-caps");
        let email_caps = check_email_caps(&hub, &all_contacts, fix, dry_run, prefix, hdr("Emails with uppercase (check-contact-email-caps)"), stats).await?;
        results.push(("check-contact-email-caps", email_caps));
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
