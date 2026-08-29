use dialoguer::{
    Confirm, Input, Select,
    theme::{ColorfulTheme, Theme},
};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    format, println,
    time::{SystemTime, UNIX_EPOCH},
};
mod decision_tree;
use decision_tree::{DecisionTree, OneToManyMap, Station};

use crate::decision_tree::{DecisionTreeNode, EdgeType};

fn get_dist_matrix(
    neighbour_map: &OneToManyMap,
    stations: &HashSet<String>,
) -> HashMap<String, HashMap<String, u32>> {
    let mut dist_matrix: HashMap<String, HashMap<String, u32>> = stations
        .iter()
        .map(|s1| {
            let inner_map: HashMap<String, u32> = stations
                .iter()
                .map(|s2| (s2.clone(), if s1 == s2 { 0 } else { u32::MAX }))
                .collect();

            (s1.clone(), inner_map)
        })
        .collect();

    for start_station in stations {
        let mut bfs: VecDeque<(&String, u32)> = VecDeque::new();
        let mut seen: HashSet<&String> = HashSet::new();

        // Initialize BFS
        bfs.push_back((&start_station, 0));
        seen.insert(&start_station);

        while let Some((current_station, current_dist)) = bfs.pop_front() {
            if let Some(inner_map) = dist_matrix.get_mut(start_station) {
                inner_map.insert(current_station.clone(), current_dist);
            }

            if let Some(neighbours) = neighbour_map.get(current_station) {
                for neighbour in neighbours {
                    if seen.contains(neighbour) {
                        continue;
                    }
                    seen.insert(neighbour);
                    bfs.push_back((neighbour, current_dist + 1));
                }
            }
        }
    }

    dist_matrix
}

#[derive(Debug, Deserialize)]
struct StationInfo {
    name: String,
    lines: Vec<String>,

    #[serde(default)]
    adjacent: Vec<AdjacentStation>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AdjacentStation {
    Name(String),
    Details(AdjacentStationDetails),
}
impl AdjacentStation {
    fn into_name(self) -> String {
        match self {
            Self::Name(name) => name,
            Self::Details(details) => details.name,
        }
    }
}

#[derive(Debug, Deserialize)]
struct AdjacentStationDetails {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Cities {
    id: String,
}

fn get_city_info(theme: &ColorfulTheme) -> Vec<StationInfo> {
    let unix_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let cities: Vec<Cities> = match ureq::get("https://networkle.fun/data/cities.json")
        .query("v", unix_time.to_string())
        .call()
    {
        Ok(mut body) => body.body_mut().read_json().unwrap(),
        Err(err) => {
            eprintln!(
                "Failed to retrieve valid city lists from networkle.fun, {}",
                err
            );
            std::process::exit(1);
        }
    };
    let available_cities: Vec<&String> = cities
        .iter()
        .map(|f| &f.id)
        .filter(|&f| f != "melbourne" && f != "newyork" && f!= "london-tube")
        .collect();
    let cities_latest_version: HashMap<String, u32> =
        match ureq::get("https://networkle.fun/data/versions.json")
            .query("v", unix_time.to_string())
            .call()
        {
            Ok(mut body) => body.body_mut().read_json().unwrap(),
            Err(err) => {
                eprintln!(
                    "Failed to retrieve valid city lists from networkle.fun, {}",
                    err
                );
                std::process::exit(1);
            }
        };

    let selection = Select::with_theme(theme)
        .with_prompt("Select a Networkle game city configuration")
        .default(0)
        .items(&available_cities[..])
        .interact()
        .unwrap();

    let chosen_city = available_cities[selection];
    let version = cities_latest_version.get(chosen_city).unwrap_or(&1u32);

    match ureq::get(format!("https://networkle.fun/data/{}.json", chosen_city))
        .query("v", version.to_string())
        .call()
    {
        Ok(mut body) => match body.body_mut().read_json() {
            Ok(data) => data,
            Err(err) => {
                println!("the error is {}", err);
                std::process::exit(1);
            }
        },
        Err(err) => {
            eprintln!(
                "Failed to retrieve valid city information from networkle.fun, {}",
                err
            );
            std::process::exit(1);
        }
    }
}

enum LineRelation {
    SharesAtLeastOneLine,
    SharesNoLines,
}
fn ask_for_feedback(
    theme: &ColorfulTheme,
    station_name: &str,
) -> Result<Option<(LineRelation, u32)>, dialoguer::Error> {
    let is_hidden_station = Confirm::with_theme(theme)
        .with_prompt(format!(
            "Recommended station to guess: {station_name}\nIs this the hidden station?"
        ))
        .default(false)
        .interact()?;

    if is_hidden_station {
        return Ok(None);
    }

    let relation_index = Select::with_theme(theme)
        .with_prompt(format!(
            "Does the hidden station share at least one line with \"{station_name}\"?"
        ))
        .items(&[
            "Station contains/is on a line which has the hidden station",
            "Station is not on the line which contains the hidden station",
        ])
        .default(0)
        .interact()?;

    let relation = match relation_index {
        0 => LineRelation::SharesAtLeastOneLine,
        1 => LineRelation::SharesNoLines,
        _ => unreachable!("Select returned an invalid index"),
    };

    let distance: u32 = Input::with_theme(theme)
        .with_prompt(format!(
            "What is the minimum distance from \"{station_name}\" to the hidden station"
        ))
        .validate_with(|distance: &u32| {
            if *distance >= 1 {
                Ok(())
            } else {
                Err("The distance must be at least 1 because this is not the hidden station.")
            }
        })
        .interact_text()?;

    Ok(Some((relation, distance)))
}

fn print_status(theme: &ColorfulTheme, message: &str) {
    let mut output = String::new();

    theme
        .format_prompt(&mut output, message)
        .expect("formatting into String should not fail");

    eprintln!("{output}");
}

fn print_error(theme: &ColorfulTheme, message: &str) {
    let mut output = String::new();
    theme
        .format_error(&mut output, message)
        .expect("formatting into String should not fail");
    eprintln!("{output}");
}

fn walk_through(tree: DecisionTree, theme: ColorfulTheme) {
    let mut cur_node: &DecisionTreeNode = &tree.root;
    loop {
        if cur_node.is_leaf() {
            print_status(
                &theme,
                &format!("Leaf node reached: {:?}", cur_node.station.name),
            );
            break;
        }

        let station_name = &cur_node.station.name;

        let Some((relation, distance)) = ask_for_feedback(&theme, station_name).unwrap() else {
            print_status(&theme, &format!("Found the hidden station: {station_name}"));
            break;
        };

        let edge = match relation {
            LineRelation::SharesAtLeastOneLine => EdgeType::SameLine(distance),
            LineRelation::SharesNoLines => EdgeType::OtherLine(distance),
        };

        cur_node = match cur_node.get_child(edge) {
            Some(next_node) => next_node,
            None => {
                print_error(
                    &theme,
                    "No matching decision-tree branch exists. \
                 Check whether the line relationship or distance was entered correctly.",
                );
                break;
            }
        }
    }
    print_status(&theme, "End of the code");
}

fn main() {
    let theme = ColorfulTheme::default();
    let city_network_json = get_city_info(&theme);

    let mut line_to_stations: OneToManyMap = OneToManyMap::new();
    let mut station_to_lines: OneToManyMap = OneToManyMap::new();
    let mut neighbour_map: OneToManyMap = OneToManyMap::new();

    for city_info in city_network_json {
        for line in &city_info.lines {
            line_to_stations
                .entry(line.clone())
                .or_default()
                .insert(city_info.name.clone());
        }

        station_to_lines
            .entry(city_info.name.clone())
            .or_default()
            .extend(city_info.lines);

        neighbour_map
            .entry(city_info.name.clone())
            .or_default()
            .extend(
                city_info
                    .adjacent
                    .into_iter()
                    .map(AdjacentStation::into_name),
            );
    }

    let stations: HashSet<String> = station_to_lines.keys().cloned().collect();
    let dist_matrix = get_dist_matrix(&neighbour_map, &stations);

    let mut station_list: Vec<Station> = Vec::new();
    for station in &stations {
        let cur_line_stations: HashSet<String> = station_to_lines
            .get(station)
            .unwrap()
            .iter()
            .flat_map(|line| line_to_stations.get(line))
            .flatten()
            .cloned()
            .collect();

        station_list.push(Station {
            name: station.clone(),
            dist_to_stations: dist_matrix.get(station).unwrap().clone(),
            cur_line_stations: cur_line_stations.clone(),
            other_line_stations: (&stations) - (&cur_line_stations),
        });
    }
    let tree = DecisionTree::new(station_list, &stations).expect("Some error has occured");
    match Select::with_theme(&theme)
        .with_prompt("Do you want the tree or the walkthrough")
        .items(["Tree", "Walkthrough"])
        .default(1)
        .interact()
        .unwrap()
    {
        0 => tree.print(),
        1 => walk_through(tree, theme),
        _ => println!("unreachable"),
    }
}
