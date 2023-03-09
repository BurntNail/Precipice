///! Module to deal with imports and exports
use std::{
    fs::{read_to_string, File},
    io::{self, Write},
    path::Path,
    time::Duration,
};

use plotly::{Plot, Histogram};

///Imports a set of traces from a CSV file
pub fn import_csv(file: impl AsRef<Path>) -> io::Result<Vec<(String, Vec<u128>)>> {
    let lines = read_to_string(file)?;
    let mut lines = lines.lines();
    let mut trace_names: Vec<String> = {
        let Some(first_trace) = lines.next() else {
            return Ok(vec![]);
        };
        first_trace.split(",")
        .map(ToString::to_string)
        .collect()
    };
        

    let mut trace_contents = vec![vec![]; trace_names.len()];
    for line in lines {
        for (i, time) in line.split(",").enumerate() {
            trace_contents[i].push(time.parse().expect("unable to parse time"));
        }
    }

    let mut out = Vec::with_capacity(trace_names.len());
    for _ in 0..trace_names.len() {
        out.push((trace_names.remove(0), trace_contents.remove(0)));
    }

    Ok(out)
}

///Exports a set of traces to a CSV file
pub fn export_csv(
    trace_name: String,
    file_name_input: impl AsRef<Path>,
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

    let (names, times): (Vec<_>, Vec<_>) = traces.into_iter().unzip();

    let mut to_be_written = String::new();
    for (i, name) in names.into_iter().enumerate() {
        if i != 0 {
            to_be_written += ",";
        }

        to_be_written += &name;
    }
    to_be_written += "\n";

    for times in times {
        for (i, time) in times.into_iter().enumerate() {
            if i != 0 {
                to_be_written += ",";
            }

            to_be_written += &time.to_string();
        }

        to_be_written += "\n";
    }

    let mut file = File::create(file_name_input)?;
    let to_be_written = to_be_written.as_bytes();
    file.write_all(to_be_written)?;

    Ok(to_be_written.len())
}

///Exports a set of traces to a plotly plot
pub fn export_html(
    trace_name: String,
    file_name_input: String,
    run_times: Vec<Duration>,
    extra_trace_file_names: Vec<String>,
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


    let mut file = File::create(file_name_input)?;
    let html = plot.to_html();
    let html = html.as_bytes();
    file.write_all(html)?;

    Ok(html.len())
}
