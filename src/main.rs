use std::collections::{HashMap, HashSet, VecDeque};
use std::{fs, println};

fn get_dist_matrix<'a>(
    neighbour_map: &HashMap<&'a String, Vec<&'a String>>,
    stations: Vec<&'a String>,
) -> HashMap<&'a String, HashMap<&'a String, u32>> {
    // 1. Change to `let mut` so we can update values during BFS
    let mut dist_matrix: HashMap<&'a String, HashMap<&'a String, u32>> = stations
        .iter()
        .map(|&s1| {
            let inner_map: HashMap<&'a String, u32> = stations
                .iter()
                // Initialize paths to self as 0, and others as u32::MAX (or leave out entirely)
                .map(|&s2| (s2, if s1 == s2 { 0 } else { u32::MAX }))
                .collect();

            (s1, inner_map)
        })
        .collect();

    // 2. Iterate through each starting station to populate its shortest paths
    for start_station in stations {
        let mut bfs: VecDeque<(&'a String, u32)> = VecDeque::new();
        let mut seen: HashSet<&'a String> = HashSet::new();

        // Initialize BFS
        bfs.push_back((start_station, 0));
        seen.insert(start_station);

        while let Some((current_station, current_dist)) = bfs.pop_front() {
            // Update the matrix value for the pair (start_station -> current_station)
            if let Some(inner_map) = dist_matrix.get_mut(start_station) {
                inner_map.insert(current_station, current_dist);
            }

            // Get neighbors from the graph adjacency list
            if let Some(neighbours) = neighbour_map.get(current_station) {
                for &neighbour in neighbours {
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

    let Ok(parsed_json) = serde_json::from_str::<Vec<Vec<String>>>(&data) else {
        return;
    };

    let mut line_to_station: HashMap<&String, HashSet<&String>> = HashMap::new();
    let mut station_to_line: HashMap<&String, &String> = HashMap::new();
    let mut neighbour_map: HashMap<&String, Vec<&String>> = HashMap::new();

    for data_tuple in &parsed_json {
        let Some(line_name) = data_tuple.get(2) else {
            println!("{:?}", data_tuple);
            continue;
        };

        line_to_station
            .entry(line_name)
            .or_default()
            .extend(data_tuple.get(0..2).unwrap().iter());

        let (Some(station1), Some(station2)) = (data_tuple.get(0), data_tuple.get(1)) else {
            continue;
        };

        station_to_line.entry(station1).insert_entry(line_name);
        station_to_line.entry(station2).insert_entry(line_name);

        neighbour_map.entry(station1).or_default().push(station2);
        neighbour_map.entry(station2).or_default().push(station1);
    }

    let stations: Vec<&String> = station_to_line.keys().map(|f| *f).collect();
    let dist_matrix = get_dist_matrix(&neighbour_map, stations);
}
