use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::{anyrun_interface::HandleResult, *};

use scrubber::ExeEntry;
use serde::Deserialize;
use std::{fs, process::Command};

#[derive(Deserialize)]
pub struct Config {
    // desktop_actions: bool,
    max_entries: usize,
    terminal_command: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // desktop_actions: false,
            max_entries: 5,
            terminal_command: Some("fish".to_owned()),
        }
    }
}

pub struct State {
    config: Config,
    entries: Vec<(ExeEntry, u64)>,
}

mod scrubber;

// const SENSIBLE_TERMINALS: &[&str] = &["alacritty", "foot", "kitty", "wezterm", "wterm"];

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

    let cmd = entry.path.as_ref().unwrap().as_os_str();
    println!("Running {:?}", cmd);
    Command::new(cmd).spawn().unwrap();
    HandleResult::Close
}

#[init]
pub fn init(config_dir: RString) -> State {
    let config: Config = match fs::read_to_string(format!("{}/run_bin.ron", config_dir)) {
        Ok(content) => ron::from_str(&content).unwrap_or_else(|why| {
            eprintln!("Error parsing applications plugin config: {}", why);
            Config::default()
        }),
        Err(why) => {
            eprintln!("Error reading applications plugin config: {}", why);
            Config::default()
        }
    };

    let entries = scrubber::scrubber().unwrap_or_else(|why| {
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
            let entry_text: Vec<char> = entry.exec.chars().collect();
            let needle: Vec<char> = input.chars().collect();
            if let Some(score) = nucleo.fuzzy_match(
                nucleo::Utf32Str::Unicode(entry_text.as_slice()),
                nucleo::Utf32Str::Unicode(needle.as_slice()),
            ) {
                Some((entry, *id, score))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| b.2.cmp(&a.2));
    entries.truncate(state.config.max_entries);

    let final_entries = entries
        .into_iter()
        .map(|(entry, id, _)| Match {
            title: entry.exec.clone().into(),
            description: ROption::RNone,
            use_pango: false,
            icon: ROption::RNone,
            id: ROption::RSome(id),
        })
        .collect::<RVec<Match>>();
    return final_entries;
}

#[info]
pub fn info() -> PluginInfo {
    PluginInfo {
        name: "Run".into(),
        icon: "application-x-appliance-symbolic".into(),
    }
}
