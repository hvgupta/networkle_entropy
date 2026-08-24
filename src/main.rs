use serde_json::{from_str, to_writer};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::BufWriter;
use std::{fs, println};

fn get_dist_matrix<'a>(
    neighbour_map: &HashMap<&'a String, Vec<&'a String>>,
    stations: &Vec<&'a String>,
) -> HashMap<&'a String, HashMap<&'a String, u32>> {
    // 1. Change to `let mut` so we can update values during BFS
    let mut dist_matrix: HashMap<&'a String, HashMap<&'a String, u32>> = stations
        .iter()
        .map(|&s1| {
            let inner_map: HashMap<&'a String, u32> = stations
                .iter()
                .map(|&s2| (s2, if s1 == s2 { 0 } else { u32::MAX }))
                .collect();

            (s1, inner_map)
        })
        .collect();

    for &start_station in stations {
        let mut bfs: VecDeque<(&'a String, u32)> = VecDeque::new();
        let mut seen: HashSet<&'a String> = HashSet::new();

        // Initialize BFS
        bfs.push_back((start_station, 0));
        seen.insert(start_station);

        while let Some((current_station, current_dist)) = bfs.pop_front() {
            if let Some(inner_map) = dist_matrix.get_mut(start_station) {
                inner_map.insert(current_station, current_dist);
            }

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

fn cat_entropy_calc(
    trgt_station_dist_matrix: &HashMap<&String, u32>,
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

fn main() {
    let Ok(data) = fs::read_to_string("./edge_json/HK.json") else {
        return;
    };

    let Ok(parsed_json) = from_str::<Vec<Vec<String>>>(&data) else {
        return;
    };

    let mut line_to_station: HashMap<&String, HashSet<&String>> = HashMap::new();
    let mut station_to_line: HashMap<&String, Vec<&String>> = HashMap::new();
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

        station_to_line.entry(station1).or_default().push(line_name);
        station_to_line.entry(station2).or_default().push(line_name);

        neighbour_map.entry(station1).or_default().push(station2);
        neighbour_map.entry(station2).or_default().push(station1);
    }

    let stations: Vec<&String> = station_to_line.keys().map(|f| *f).collect();
    let dist_matrix = get_dist_matrix(&neighbour_map, &stations);

    let mut station_to_entropy: HashMap<&String, f32> = HashMap::new();

    for &station in &stations {
        let Some(cur_lines) = station_to_line.get(station) else {
            continue;
        };
        let cur_lines_stations: HashSet<&String> = cur_lines
            .iter()
            .flat_map(|&line| line_to_station.get(line))
            .flatten()
            .copied()
            .collect();

        let other_lines =
            (&stations.iter().copied().collect::<HashSet<&String>>()) - (&cur_lines_stations);

        let trgt_station_dist_matrix = &dist_matrix.get(station).unwrap();

        let entropy: f32 = cat_entropy_calc(
            trgt_station_dist_matrix,
            &cur_lines_stations,
            station.len() as f32,
        ) + cat_entropy_calc(
            trgt_station_dist_matrix,
            &other_lines,
            stations.len() as f32,
        );

        station_to_entropy.entry(station).insert_entry(entropy);
    }

    let file = File::create("./response/HK_entropy.json").unwrap();
    let writer = BufWriter::new(file);
    let _ = to_writer(writer, &station_to_entropy);
}
