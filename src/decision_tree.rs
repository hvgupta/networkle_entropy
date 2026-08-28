use std::collections::{HashMap, HashSet};

pub type OneToManyMap = HashMap<String, Vec<String>>;

fn cat_entropy_calc(
    trgt_station_dist_matrix: &HashMap<String, u32>,
    stations_set: &HashSet<&String>,
    num_stations: f32,
) -> f32 {
    let mut entropy = 0f32;
    let mut freq_map: HashMap<u32, u32> = HashMap::new();

    for &station in stations_set {
        *(freq_map
            .entry(*trgt_station_dist_matrix.get(station).unwrap())
            .or_insert(0)) += 1;
    }

    for &value in freq_map.values() {
        if value == 1u32 {
            continue;
        }

        let value_f32 = value as f32;

        entropy += (value_f32 / num_stations)
            * ((value_f32.ln() / value_f32)
                + ((value_f32 - 1f32) / value_f32) * (value_f32 / (value_f32 - 1f32)).ln());
    }

    entropy
}

fn one_step_entropy<'a>(
    stations: &'a [Station],
    valid_stations: &HashSet<String>,
) -> Option<(&'a Station, f32)> {
    let mut lowest_station: Option<(&'a Station, f32)> = None;

    for station in stations
        .iter()
        .filter(|station| valid_stations.contains(&station.name))
    {
        let cur_line_stations: HashSet<&String> = station
            .cur_line_stations
            .intersection(valid_stations)
            .collect();

        let other_line_stations: HashSet<&String> = station
            .other_line_stations
            .intersection(valid_stations)
            .collect();

        let entropy = cat_entropy_calc(
            &station.dist_to_stations,
            &cur_line_stations,
            valid_stations.len() as f32,
        ) + cat_entropy_calc(
            &station.dist_to_stations,
            &other_line_stations,
            valid_stations.len() as f32,
        );

        match lowest_station {
            None => lowest_station = Some((station, entropy)),
            Some((_, lowest_entropy)) if entropy > lowest_entropy => {
                lowest_station = Some((station, entropy));
            }
            _ => {}
        }
    }

    lowest_station
}

#[derive(Debug, Clone)]
pub struct Station {
    pub name: String,
    pub dist_to_stations: HashMap<String, u32>,
    pub cur_line_stations: HashSet<String>,
    pub other_line_stations: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeType {
    SameLine(u32),
    OtherLine(u32),
}

#[derive(Debug)]
struct DecisionTreeNode {
    station: Station,
    children: HashMap<EdgeType, DecisionTreeNode>,
}

impl DecisionTreeNode {
    pub fn new(
        stations: Vec<Station>,
        valid_stations: &HashSet<String>,
    ) -> Option<DecisionTreeNode> {
        let Some((selected_station, _)) = one_step_entropy(&stations, valid_stations) else {
            return None;
        };

        let selected_station = selected_station.clone();

        let station_by_name: HashMap<&str, &Station> = stations
            .iter()
            .map(|station| (station.name.as_str(), station))
            .collect();

        let mut children: HashMap<EdgeType, DecisionTreeNode> = HashMap::new();

        let dist_to_cur_name = map_distances_to_stations(
            valid_stations,
            &selected_station.cur_line_stations,
            &selected_station.dist_to_stations,
        );
        generate_category_children(
            &selected_station,
            &station_by_name,
            &mut children,
            dist_to_cur_name,
            EdgeType::SameLine,
        );

        let dist_to_other_name = map_distances_to_stations(
            valid_stations,
            &selected_station.other_line_stations,
            &selected_station.dist_to_stations,
        );
        generate_category_children(
            &selected_station,
            &station_by_name,
            &mut children,
            dist_to_other_name,
            EdgeType::OtherLine,
        );

        Some(DecisionTreeNode {
            station: selected_station,
            children,
        })
    }

    fn print_recursive(&self, depth: usize, incoming_edge: Option<&EdgeType>) {
        let indent = "| ".repeat(depth);

        match incoming_edge {
            None => {
                println!("{indent}{}", self.station.name);
            }
            Some(EdgeType::SameLine(dist)) => {
                println!("{indent}└─ SameLine({dist}) -> {}", self.station.name);
            }
            Some(EdgeType::OtherLine(dist)) => {
                println!("{indent}└─ OtherLine({dist}) -> {}", self.station.name);
            }
        }

        let mut ordered_children: Vec<(&EdgeType, &DecisionTreeNode)> =
            self.children.iter().collect();

        ordered_children.sort_by_key(|(edge, _)| match edge {
            EdgeType::SameLine(dist) => (0_u8, *dist),
            EdgeType::OtherLine(dist) => (1_u8, *dist),
        });

        for (edge, child) in ordered_children {
            child.print_recursive(depth + 1, Some(edge));
        }
    }
}

fn generate_category_children<'a>(
    selected_station: &Station,
    station_by_name: &HashMap<&'a str, &'a Station>,
    children: &mut HashMap<EdgeType, DecisionTreeNode>,
    dist_to_name: HashMap<u32, Vec<String>>,
    edge_type: fn(u32) -> EdgeType,
) {
    for (dist, station_names) in dist_to_name {
        let next_valid_stations: HashSet<String> = station_names
            .iter()
            .filter(|name| name.as_str() != selected_station.name.as_str())
            .cloned()
            .collect();

        let child_stations: Vec<Station> = station_names
            .iter()
            .filter(|name| name.as_str() != selected_station.name.as_str())
            .map(|name| {
                Station::clone(
                    *station_by_name
                        .get(name.as_str())
                        .expect("station name must exist in station_by_name"),
                )
            })
            .collect();

        if let Some(child) = DecisionTreeNode::new(child_stations, &next_valid_stations) {
            children.insert(edge_type(dist), child);
        }
    }
}

fn map_distances_to_stations(
    valid_stations: &HashSet<String>,
    category_stations: &HashSet<String>,
    dist_to_stations: &HashMap<String, u32>,
) -> HashMap<u32, Vec<String>> {
    let mut dist_to_names: HashMap<u32, Vec<String>> = HashMap::new();

    for station_name in category_stations.intersection(valid_stations) {
        let dist = *dist_to_stations
            .get(station_name)
            .expect("station in category_stations must have a distance");

        dist_to_names
            .entry(dist)
            .or_default()
            .push(station_name.clone());
    }

    dist_to_names
}

#[derive(Debug)]
pub struct DecisionTree {
    root: DecisionTreeNode,
}
impl DecisionTree {
    pub fn new(stations: Vec<Station>, valid_stations: &HashSet<String>) -> DecisionTree {
        return DecisionTree {
            root: DecisionTreeNode::new(stations, valid_stations).unwrap(),
        };
    }
    pub fn print(&self) {
        self.root.print_recursive(0, None);
    }
}
