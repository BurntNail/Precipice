///! Module to deal with imports and exports
use std::fmt::Display;
use std::{
    fs::{read_to_string, File},
    io::{self, Write},
    path::Path,
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

///Getting multiple traces from multiple files
fn get_traces(
    trace_file_names: impl IntoIterator<Item = impl AsRef<Path>>,
    trace: Option<(String, Vec<u128>)>,
) -> io::Result<Vec<(String, Vec<u128>)>> {
    let mut traces: Vec<(String, Vec<u128>)> = trace_file_names
        .into_iter()
        .map(import_csv)
        .collect::<io::Result<Vec<Vec<(String, Vec<u128>)>>>>()?
        .into_iter()
        .flatten()
        .collect();
    if let Some((name, times)) = trace {
        traces.push((name, times));
    }
    Ok(traces)
}

///Exports a set of traces to a CSV file
///
/// # Errors
///
/// Can have errors if we fail to create a file or write to it, or if we fail to read the traces
pub fn export_csv(
    trace: Option<(String, Vec<u128>)>,
    file_name_input: impl AsRef<Path> + Display,
    extra_trace_file_names: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<usize> {
    let traces = get_traces(extra_trace_file_names, trace)?;
    export_csv_no_file_input(file_name_input, traces)
}

///Exports a set of traces to a CSV file
///
/// # Errors
///
/// Can have errors if we fail to create a file or write to it
pub fn export_csv_no_file_input(
    file_name_input: impl AsRef<Path> + Display,
    traces: Vec<(String, Vec<u128>)>,
) -> io::Result<usize> {
    let mut to_be_written = String::new();

    for (name, times) in traces {
        to_be_written += &name;
        for time in times {
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
///
/// # Errors
///
/// Can have errors if we fail to create a file or write to it, or if we fail to read the traces
pub fn export_html(
    trace: Option<(String, Vec<u128>)>,
    file_name_input: impl AsRef<Path> + Display,
    extra_trace_file_names: impl IntoIterator<Item = impl AsRef<Path>>,
) -> io::Result<usize> {
    let traces = get_traces(extra_trace_file_names, trace)?;
    export_html_no_file_input(file_name_input, traces)
}

///Exports a set of traces to a CSV file
///
/// # Errors
///
/// Can have errors if we fail to create a file or write to it
pub fn export_html_no_file_input(
    file_name_input: impl AsRef<Path> + Display,
    traces: Vec<(String, Vec<u128>)>,
) -> io::Result<usize> {
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
