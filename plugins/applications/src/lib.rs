use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::{anyrun_interface::HandleResult, *};
use scrubber::DesktopEntry;
use serde::Deserialize;
use std::{env, fs, process::Command};

#[derive(Deserialize)]
pub struct Config {
    desktop_actions: bool,
    max_entries: usize,
    terminal: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop_actions: false,
            max_entries: 5,
            terminal: None,
        }
    }
}

pub struct State {
    config: Config,
    entries: Vec<(DesktopEntry, u64)>,
}

mod scrubber;

const SENSIBLE_TERMINALS: &[&str] = &["alacritty", "foot", "kitty", "wezterm", "wterm"];

#[handler]
pub fn handler(selection: Match, state: &State) -> HandleResult {
    let entry = state
        .entries
        .iter()
        .find_map(|(entry, id)| {
            if *id == selection.id.unwrap() {
                Some(entry)
            } else {
                None
            }
        })
        .unwrap();

    if entry.term {
        match &state.config.terminal {
            Some(term) => {
                if let Err(why) = Command::new(term).arg("-e").arg(&entry.exec).spawn() {
                    eprintln!("Error running desktop entry: {}", why);
                }
            }
            None => {
                for term in SENSIBLE_TERMINALS {
                    if Command::new(term)
                        .arg("-e")
                        .arg(&entry.exec)
                        .spawn()
                        .is_ok()
                    {
                        break;
                    }
                }
            }
        }
    } else if let Err(why) = {
        let current_dir = &env::current_dir().unwrap();

        Command::new("sh")
            .arg("-c")
            .arg(&entry.exec)
            .current_dir(if let Some(path) = &entry.path {
                if path.exists() {
                    path
                } else {
                    current_dir
                }
            } else {
                current_dir
            })
            .spawn()
    } {
        eprintln!("Error running desktop entry: {}", why);
    }

    HandleResult::Close
}

#[init]
pub fn init(config_dir: RString) -> State {
    let config: Config = match fs::read_to_string(format!("{}/applications.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!("Error parsing applications plugin config: {}", why);
            Config::default()
        }),
        Err(why) => {
            eprintln!("Error reading applications plugin config: {}", why);
            Config::default()
        }
    };

    let entries = scrubber::scrubber(&config).unwrap_or_else(|why| {
        eprintln!("Failed to load desktop entries: {}", why);
        Vec::new()
    });

    State { config, entries }
}

#[get_matches]
pub fn get_matches(input: RString, state: &State) -> RVec<Match> {
    let mut nucleo = nucleo::Matcher::new(nucleo::Config::DEFAULT.match_paths());
    let mut entries = state
        .entries
        .iter()
        .filter_map(|(entry, id)| {
            let entry_text: Vec<char> = entry.name.chars().collect();
            let needle: Vec<char> = input.chars().collect();
            let app_score = match &entry.desc {
                None => nucleo
                    .fuzzy_match(
                        nucleo::Utf32Str::Unicode(entry_text.as_slice()),
                        nucleo::Utf32Str::Unicode(needle.as_slice()),
                    )
                    .unwrap_or(0),
                Some(val) => {
                    let needle: Vec<char> = format!("{} {}", &val, &entry.name).chars().collect();
                    nucleo
                        .fuzzy_match(
                            nucleo::Utf32Str::Unicode(entry_text.as_slice()),
                            nucleo::Utf32Str::Unicode(needle.as_slice()),
                        )
                        .unwrap_or(0)
                }
            };

            let keyword_score = entry
                .keywords
                .iter()
                .map(|keyword| {
                    let keyword: Vec<char> = keyword.chars().collect();
                    nucleo
                        .fuzzy_match(
                            nucleo::Utf32Str::Unicode(keyword.as_slice()),
                            nucleo::Utf32Str::Unicode(needle.as_slice()),
                        )
                        .unwrap_or(0)
                })
                .sum::<u16>();
            let mut score = (app_score * 25 + keyword_score) as i64 - entry.offset;

            // prioritize actions
            if entry.desc.is_some() {
                score = score * 2;
            }

            if score > 0 {
                Some((entry, *id, score))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| b.2.cmp(&a.2));

    entries.truncate(state.config.max_entries);
    entries
        .into_iter()
        .map(|(entry, id, _)| Match {
            title: entry.name.clone().into(),
            description: entry.desc.clone().map(|desc| desc.into()).into(),
            use_pango: false,
            icon: ROption::RSome(entry.icon.clone().into()),
            id: ROption::RSome(id),
        })
        .collect()
}

#[info]
pub fn info() -> PluginInfo {
    PluginInfo {
        name: "Applications".into(),
        icon: "application-x-executable".into(),
    }
}
