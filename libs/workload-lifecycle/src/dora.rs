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

/// Select per-run endpoints for zero ports and rewrite the two native commands.
/// Explicit coordinator references must agree; the two resolved ports must differ.
pub fn resolve_ports(
    coordinator: &mut [String],
    daemon: &mut [String],
) -> Result<[bool; 2], String> {
    let read = |args: &[String], flag| {
        option(args, flag)?
            .parse::<u16>()
            .map_err(|e| e.to_string())
    };
    let mut ports = [
        read(coordinator, "--port")?,
        read(daemon, "--local-listen-port")?,
    ];
    if read(daemon, "--coordinator-port")? != ports[0] {
        return Err("Dora app coordinator endpoints disagree".into());
    }
    let automatic = ports.map(|port| port == 0);
    let mut listeners = Vec::new();
    for port in &mut ports {
        if *port == 0 {
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
                .map_err(|e| e.to_string())?;
            *port = listener.local_addr().map_err(|e| e.to_string())?.port();
            listeners.push(listener);
        }
    }
    if ports[0] == ports[1] {
        return Err("Dora ports must differ".into());
    }
    fn replace(args: &mut [String], flag: &str, port: u16) {
        let prefix = format!("{flag}=");
        for index in 0..args.len() {
            if args[index] == flag {
                args[index + 1] = port.to_string();
            } else if args[index].starts_with(&prefix) {
                args[index] = format!("{prefix}{port}");
            }
        }
    }
    replace(coordinator, "--port", ports[0]);
    replace(daemon, "--coordinator-port", ports[0]);
    replace(daemon, "--local-listen-port", ports[1]);
    Ok(automatic)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ports_are_automatic_or_preserved_and_invalid_pairs_fail() {
        // Arrange
        let coordinator = |port| {
            vec![
                "dora".into(),
                "coordinator".into(),
                format!("--port={port}"),
            ]
        };
        let daemon = |target, port| {
            vec![
                "dora".into(),
                "daemon".into(),
                format!("--coordinator-port={target}"),
                format!("--local-listen-port={port}"),
            ]
        };
        let (mut c, mut d) = (coordinator(0), daemon(0, 0));
        // Act
        let automatic = resolve_ports(&mut c, &mut d).unwrap();
        // Assert
        assert_eq!(automatic, [true, true]);
        assert_eq!(port(&c, "--port"), port(&d, "--coordinator-port"));
        assert_ne!(port(&c, "--port"), port(&d, "--local-listen-port"));
        // Act / Assert: explicit settings remain literal configuration.
        let (mut c, mut d) = (coordinator(6013), daemon(6013, 6014));
        assert_eq!(resolve_ports(&mut c, &mut d).unwrap(), [false, false]);
        assert_eq!(port(&c, "--port"), Ok(6013));
        assert_eq!(port(&d, "--local-listen-port"), Ok(6014));
        assert!(resolve_ports(&mut coordinator(6013), &mut daemon(6014, 6015)).is_err());
        assert!(resolve_ports(&mut coordinator(6013), &mut daemon(6013, 6013)).is_err());
    }

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
