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

pub async fn cmd_list(emails: bool, labels: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let mut fields = vec!["names", "phoneNumbers"];
    if emails { fields.push("emailAddresses"); }
    let contacts = fetch_all_contacts(&hub, &fields).await?;

    for person in &contacts {
        let name = person_display_name(person);

        if labels {
            if let Some(nums) = &person.phone_numbers {
                for pn in nums {
                    let phone = pn.value.as_deref().unwrap_or("");
                    let label = {
                        let l = get_phone_label(pn);
                        if l.is_empty() { "<no label>" } else { l }
                    };
                    if emails {
                        println!("{} | {} | {} [{}]", name, person_email(person), phone, label);
                    } else {
                        println!("{} | {} [{}]", name, phone, label);
                    }
                }
            } else if emails {
                println!("{} | {}", name, person_email(person));
            } else {
                println!("{}", name);
            }
        } else {
            let phone = person
                .phone_numbers
                .as_ref()
                .and_then(|phones| phones.first())
                .and_then(|p| p.value.as_deref())
                .unwrap_or("");

            if emails {
                let email = person_email(person);
                if !email.is_empty() || !phone.is_empty() {
                    println!("{} | {} | {}", name, email, phone);
                } else {
                    println!("{}", name);
                }
            } else if !phone.is_empty() {
                println!("{} | {}", name, phone);
            } else {
                println!("{}", name);
            }
        }
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

pub async fn cmd_check_english(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_issues(&hub, &contacts, |name| !is_english_name(name), fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_caps(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_issues(&hub, &contacts, |name| is_all_caps(name), fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_first_capital_letter(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_issues(&hub, &contacts, |name| !starts_with_capital(name), fix, dry_run, "", None, false).await?;
    Ok(())
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

pub async fn cmd_check_name_order(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses"]).await?;
    check_name_order(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
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
                if dry_run {
                    eprintln!("(dry-run) would prompt for action");
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
                            if let Some(group_rn) = prompt_label_autocomplete(label_names, user_groups)? {
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

pub async fn cmd_check_duplicate_phones(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let contacts = fetch_all_contacts(&hub, &["names", "phoneNumbers"]).await?;
    check_duplicate_phones(&hub, &contacts, fix, dry_run, "", None, false).await?;
    Ok(())
}

pub async fn cmd_check_email() -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn cmd_check_email_caps(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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

pub async fn cmd_check_duplicate_emails(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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
    let contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "memberships"]).await?;

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

fn prompt_label_autocomplete(
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
    match rl.readline("  Label (tab to complete): ") {
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
                eprintln!("  Unknown label \"{}\". Available: {}", trimmed,
                    label_names.join(", "));
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

pub async fn cmd_check_labels_nophone(fix: bool, dry_run: bool) -> Result<(), Box<dyn std::error::Error>> {
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

    for (i, person) in matches.iter().enumerate() {
        if i > 0 { println!("\n{}", "=".repeat(60)); }
        print_person_details(person);
    }
    Ok(())
}

fn print_person_details(person: &google_people1::api::Person) {
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
        let labels: Vec<&str> = memberships.iter().filter_map(|m| {
            m.contact_group_membership.as_ref().and_then(|cgm| {
                cgm.contact_group_resource_name.as_deref()
            })
        }).filter(|rn| *rn != "contactGroups/myContacts").collect();
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
        if group.group_type.as_deref() == Some("USER_CONTACT_GROUP") {
            let name = group.name.as_deref().unwrap_or("<unnamed>");
            let count = group.member_count.unwrap_or(0);
            println!("{} ({})", name, count);
        }
    }
    Ok(())
}

pub async fn cmd_check_all(fix: bool, dry_run: bool, stats: bool, country: &str) -> Result<(), Box<dyn std::error::Error>> {
    let hub = build_hub().await?;
    let all_contacts = fetch_all_contacts(&hub, &["names", "emailAddresses", "phoneNumbers", "memberships"]).await?;

    let mut results: Vec<(&str, usize)> = Vec::new();

    let prefix = if stats { "" } else { "  " };
    let hdr = |s: &'static str| -> Option<&'static str> { if stats { None } else { Some(s) } };

    let non_english = check_name_issues(
        &hub, &all_contacts, |name| !is_english_name(name),
        fix, dry_run, prefix, hdr("Non-English names (check-name-english)"), stats,
    ).await?;
    results.push(("check-name-english", non_english));

    let all_caps = check_name_issues(
        &hub, &all_contacts, |name| is_all_caps(name),
        fix, dry_run, prefix, hdr("All-caps names (check-name-caps)"), stats,
    ).await?;
    results.push(("check-name-caps", all_caps));

    let country_owned = country.to_string();
    let no_country = check_phone_issues(
        &hub, &all_contacts,
        |v| is_fixable_phone(v) && !has_country_code(v),
        move |v| add_country_code(v, &country_owned),
        fix, dry_run, prefix, hdr("Phones missing country code (check-phone-countrycode)"), stats,
    ).await?;
    results.push(("check-phone-countrycode", no_country));

    let country_owned2 = country.to_string();
    let bad_format = check_phone_issues(
        &hub, &all_contacts,
        |v| is_fixable_phone(v) && !is_correct_phone_format(v),
        move |v| fix_phone_format(v, &country_owned2),
        fix, dry_run, prefix, hdr("Phones not in +CC-NUMBER format (check-phone-format)"), stats,
    ).await?;
    results.push(("check-phone-format", bad_format));

    let first_cap = check_name_issues(
        &hub, &all_contacts, |name| !starts_with_capital(name),
        fix, dry_run, prefix, hdr("Names not starting with capital letter (check-name-first-capital-letter)"), stats,
    ).await?;
    results.push(("check-name-first-capital-letter", first_cap));

    let name_order = check_name_order(&hub, &all_contacts, fix, dry_run, prefix, hdr("Reversed name order (check-name-order)"), stats).await?;
    results.push(("check-name-order", name_order));

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

    let no_label = check_no_label(&hub, &all_contacts, fix, dry_run, prefix, hdr("Contacts without label (check-contact-no-label)"), stats, &user_groups, &label_names).await?;
    results.push(("check-contact-no-label", no_label));

    let phone_no_label = check_phone_no_label(&hub, &all_contacts, fix, dry_run, prefix, hdr("Phones without label (check-phone-no-label)"), stats).await?;
    results.push(("check-phone-no-label", phone_no_label));

    let phone_label_eng = check_phone_label_english(&hub, &all_contacts, fix, dry_run, prefix, hdr("Non-English phone labels (check-phone-label-english)"), stats).await?;
    results.push(("check-phone-label-english", phone_label_eng));

    results.push(("check-email", check_invalid_emails(&all_contacts, prefix, hdr("Invalid emails (check-email)"), stats)));

    let email_caps = check_email_caps(&hub, &all_contacts, fix, dry_run, prefix, hdr("Emails with uppercase (check-email-caps)"), stats).await?;
    results.push(("check-email-caps", email_caps));

    let dup_phones = check_duplicate_phones(&hub, &all_contacts, fix, dry_run, prefix, hdr("Duplicate phone numbers (check-duplicate-phones)"), stats).await?;
    results.push(("check-duplicate-phones", dup_phones));

    let dup_emails = check_duplicate_emails(&hub, &all_contacts, fix, dry_run, prefix, hdr("Duplicate email addresses (check-duplicate-emails)"), stats).await?;
    results.push(("check-duplicate-emails", dup_emails));

    // Check for empty labels (contact groups) — separate API call
    let all_groups = fetch_all_contact_groups(&hub).await?;

    let empty: Vec<_> = all_groups.iter().filter(|g| {
        g.member_count.unwrap_or(0) == 0
            && g.group_type.as_deref() == Some("USER_CONTACT_GROUP")
    }).collect();
    if !stats && !empty.is_empty() {
        println!("=== Empty labels (check-labels-nophone) ({}) ===", empty.len());
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
    results.push(("check-labels-nophone", empty.len()));

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
