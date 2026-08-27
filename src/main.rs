use serde_json::from_str;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
mod decision_tree;
use decision_tree::{DecisionTree, OneToManyMap, Station};

fn get_dist_matrix(
    neighbour_map: &OneToManyMap,
    stations: &HashSet<String>,
) -> HashMap<String, HashMap<String, u32>> {
    // 1. Change to `let mut` so we can update values during BFS
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

fn main() {
    let Ok(data) = fs::read_to_string("./edge_json/HK.json") else {
        return;
    };

    let Ok(parsed_json) = from_str::<Vec<Vec<String>>>(&data) else {
        return;
    };

    let mut line_to_stations: HashMap<String, HashSet<String>> = HashMap::new();
    let mut station_to_lines: OneToManyMap = OneToManyMap::new();
    let mut neighbour_map: OneToManyMap = OneToManyMap::new();

    for data_tuple in &parsed_json {
        let Some(line_name) = data_tuple.get(2) else {
            println!("{:?}", data_tuple);
            continue;
        };

        line_to_stations
            .entry(line_name.clone())
            .or_default()
            .extend(data_tuple.get(0..2).unwrap().iter().cloned());

        let (Some(station1), Some(station2)) = (data_tuple.get(0), data_tuple.get(1)) else {
            continue;
        };

        station_to_lines
            .entry(station1.clone())
            .or_default()
            .push(line_name.clone());
        station_to_lines
            .entry(station2.clone())
            .or_default()
            .push(line_name.clone());

        neighbour_map
            .entry(station1.clone())
            .or_default()
            .push(station2.clone());
        neighbour_map
            .entry(station2.clone())
            .or_default()
            .push(station1.clone());
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
    let tree = DecisionTree::new(station_list, &stations);
    tree.print();
}
