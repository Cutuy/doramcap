//! Dora command-line values read from ordinary app launch definitions.

/// Read one explicit native CLI option; duplicate, missing and empty values fail.
pub fn option<'a>(argv: &'a [String], name: &str) -> Result<&'a str, String> {
    let prefix = format!("{name}=");
    let mut found = None;
    for (index, arg) in argv.iter().enumerate() {
        let value = if arg == name {
            Some(
                argv.get(index + 1)
                    .map(String::as_str)
                    .ok_or_else(|| format!("missing {name} value"))?,
            )
        } else {
            arg.strip_prefix(&prefix)
        };
        if let Some(value) = value
            && (value.is_empty() || value.starts_with("--") || found.replace(value).is_some())
        {
            return Err(format!("ambiguous or empty {name}"));
        }
    }
    found.ok_or_else(|| format!("Dora app command must declare {name}"))
}

/// Read a nonzero TCP port from an app's native CLI option.
pub fn port(argv: &[String], name: &str) -> Result<u16, String> {
    option(argv, name)?
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| format!("{name} must be in 1..=65535"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_options_accept_both_forms_and_reject_ambiguous_endpoints() {
        // Arrange / Act / Assert
        for args in [
            vec!["dora", "coordinator", "--port", "6013"],
            vec!["dora", "coordinator", "--port=6013"],
        ] {
            assert_eq!(
                port(
                    &args.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                    "--port"
                ),
                Ok(6013)
            );
        }
        for args in [
            vec!["--port"],
            vec!["--port", "--store"],
            vec!["--port=0"],
            vec!["--port=65536"],
            vec!["--port=6013", "--port", "6014"],
        ] {
            assert!(
                port(
                    &args.into_iter().map(str::to_owned).collect::<Vec<_>>(),
                    "--port"
                )
                .is_err()
            );
        }
    }
}
