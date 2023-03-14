use std::fmt::Display;
///! Module to deal with imports and exports
use std::{
    fs::{read_to_string, File},
    io::{self, Write},
    path::Path,
    time::Duration,
};

use plotly::{Histogram, Plot};

///Imports a set of traces from a CSV file
///
/// # Errors
///
/// Can fail if we fail to read the file using [`read_to_string`]
pub fn import_csv(file: impl AsRef<Path>) -> io::Result<Vec<(String, Vec<u128>)>> {
    let lines = read_to_string(file)?;
    if lines.trim().is_empty() {
        return Ok(vec![]);
    }
    let no_lines = lines.lines().count();
    let lines = lines.lines();

    let mut trace_contents = vec![(String::new(), vec![]); no_lines];
    for (i, line) in lines.enumerate() {
        for (j, time) in line.split(',').enumerate() {
            if j == 0 {
                trace_contents[i].0 = time.to_string();
            } else {
                trace_contents[i]
                    .1
                    .push(time.parse().expect("unable to parse time"));
            }
        }
    }

    Ok(trace_contents)
}

///Exports a set of traces to a CSV file
pub fn export_csv(
    trace_name: String,
    file_name_input: impl AsRef<Path> + Display,
    run_times: Vec<Duration>,
    extra_trace_file_names: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<usize> {
    let mut traces: Vec<(String, Vec<u128>)> = extra_trace_file_names
        .into_iter()
        .map(import_csv)
        .collect::<io::Result<Vec<Vec<(String, Vec<u128>)>>>>()?
        .into_iter()
        .flatten()
        .collect();
    traces.push((
        trace_name,
        run_times.into_iter().map(|x| x.as_micros()).collect(),
    )); //TODO: have one closure thingie for timing

    let mut to_be_written = String::new();

    for (name, times) in traces {
        to_be_written += &name;
        for time in times.into_iter() {
            to_be_written += ",";
            to_be_written += &time.to_string();
        }

        to_be_written += "\n";
    }

    let mut file = File::create(format!("{file_name_input}.csv"))?;
    let to_be_written = to_be_written.as_bytes();
    file.write_all(to_be_written)?;

    Ok(to_be_written.len())
}

///Exports a set of traces to a plotly plot
pub fn export_html(
    trace_name: String,
    file_name_input: impl AsRef<Path> + Display,
    run_times: Vec<Duration>,
    extra_trace_file_names: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<usize> {
    let mut traces: Vec<(String, Vec<u128>)> = extra_trace_file_names
        .into_iter()
        .map(import_csv)
        .collect::<io::Result<Vec<Vec<(String, Vec<u128>)>>>>()?
        .into_iter()
        .flatten()
        .collect();
    traces.push((
        trace_name,
        run_times.into_iter().map(|x| x.as_micros()).collect(),
    )); //TODO: have one closure thingie for timin

    let mut plot = Plot::new();
    for (name, trace) in traces {
        plot.add_trace(Histogram::new(trace).name(name));
    }

    let mut file = File::create(format!("{file_name_input}.html"))?;
    let html = plot.to_html();
    let html = html.as_bytes();
    file.write_all(html)?;

    Ok(html.len())
}
