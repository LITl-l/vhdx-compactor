//! Parsing of `wsl.exe -l -v` console output.
//!
//! `wsl.exe` prints a fixed-width table; the default distro is marked with a
//! leading `*`. We only need the name and running state here — the vhdx path is
//! resolved separately from the registry.

/// A single row from `wsl -l -v` (name + running state only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistroListing {
    pub name: String,
    pub running: bool,
}

/// Parse the decoded text of `wsl -l -v`. Skips the header row and blank lines.
/// The leading `*` (default-distro marker) is stripped from the name column.
pub fn parse_wsl_list(text: &str) -> Vec<DistroListing> {
    text.lines()
        .filter_map(|raw| {
            let line = raw.trim_end();
            if line.trim().is_empty() {
                return None;
            }
            // Strip the optional default-distro marker before splitting columns.
            let line = line.trim_start().strip_prefix('*').unwrap_or(line);
            let mut cols = line.split_whitespace();
            let name = cols.next()?.to_string();
            if name == "NAME" {
                return None; // header row
            }
            let state = cols.next().unwrap_or_default();
            Some(DistroListing {
                name,
                running: state.eq_ignore_ascii_case("Running"),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
  NAME            STATE           VERSION
* Ubuntu          Running         2
  Debian          Stopped         2
  docker-desktop  Stopped         2
";

    #[test]
    fn parses_names_and_state() {
        let got = parse_wsl_list(SAMPLE);
        assert_eq!(
            got,
            vec![
                DistroListing {
                    name: "Ubuntu".into(),
                    running: true
                },
                DistroListing {
                    name: "Debian".into(),
                    running: false
                },
                DistroListing {
                    name: "docker-desktop".into(),
                    running: false
                },
            ]
        );
    }

    #[test]
    fn ignores_header_and_blank_lines() {
        assert!(parse_wsl_list("  NAME  STATE  VERSION\n\n").is_empty());
    }

    #[test]
    fn handles_default_marker_with_leading_space() {
        // wsl emits "* Name" with the star indented under the marker column.
        let got = parse_wsl_list("  NAME  STATE  VERSION\n* Ubuntu-24.04  Running  2\n");
        assert_eq!(
            got,
            vec![DistroListing {
                name: "Ubuntu-24.04".into(),
                running: true
            }]
        );
    }
}
