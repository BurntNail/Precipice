//! Module to deal with imports and exports

use std::{
    fmt::Display,
    fs::{read_to_string, File},
    io::{self, Write},
    path::Path,
};
use clap::ValueEnum;
use crate::bencher::DEFAULT_RUNS;
use plotly::{Histogram, Plot};

///Imports a set of traces from a CSV file
///
/// # Errors
///
/// Can fail if we fail to read the file using [`read_to_string`]
pub fn import_csv(file: impl AsRef<Path>) -> io::Result<Vec<(String, Vec<u128>)>> {
    let lines = read_to_string(file)?; //read in the csv file
    if lines.trim().is_empty() {
        //if it is empty (need to trim in case of extra newlines etc), just return an empty list
        return Ok(vec![]);
    }
    let no_lines = lines.lines().count(); //have to get lines twice, as count consumes
    let lines = lines.lines();

    let mut trace_contents: Vec<(String, Vec<u128>)> = Vec::with_capacity(no_lines);
    for line in lines {
        let mut title = String::new();
        let mut contents = Vec::with_capacity(
            trace_contents
                .first()
                .map_or(DEFAULT_RUNS, |(_, v)| v.len()),
        ); //make a new vec with the capacity of the first one, or if we don't have one yet, use DEFAULT_RUNS
        for (j, time) in line.split(',').enumerate() {
            if j == 0 {
                title = time.to_string(); //here, it isn't a time, its a title - the first item is the title
            } else {
                contents.push(time.parse().expect("unable to parse time")); //here, it is a time, so we need to parse it.
            }
        }
        trace_contents.push((title, contents));
    }

    Ok(trace_contents)
}

///Getting multiple traces from multiple files in CSV format
///
/// # Errors
/// If we can't do something with the file
pub fn get_traces(
    trace_file_names: impl IntoIterator<Item = impl AsRef<Path>>,
    trace: Option<(String, Vec<u128>)>,
) -> io::Result<Vec<(String, Vec<u128>)>> {
    let mut traces: Vec<(String, Vec<u128>)> = trace_file_names
        .into_iter() //for each trace
        .map(import_csv) //import it
        .collect::<io::Result<Vec<Vec<(String, Vec<u128>)>>>>()? //collect any results and bubble
        .into_iter() //make that back into an iterator
        .flatten() //flatten it - Vec<Vec<T>> to a flat Vec<T>
        .collect(); //then get that into a Vec<T>
    if let Some((name, times)) = trace {
        //if we got a trace to start with
        traces.push((name, times)); //add it
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
    let traces = get_traces(extra_trace_file_names, trace)?; //get the traces from the file and provided
    export_csv_no_file_input(file_name_input, traces) //export
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
    let mut to_be_written = String::new(); //string with space to be written to

    for (name, times) in traces {
        to_be_written += &name;
        for time in times {
            to_be_written += ",";
            to_be_written += &time.to_string();
        }
        to_be_written += "\n";
    } //manually write a csv - title,time1,time2,time3 etc

    let mut file = File::create(format!("{file_name_input}.csv"))?; //make a file
    let to_be_written = to_be_written.as_bytes(); //get the bytes to be written
    file.write_all(to_be_written)?; //write them all

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
    let traces = get_traces(extra_trace_file_names, trace)?; //get the traces from the file and provided
    export_html_no_file_input(file_name_input, traces) //and export them
}

///Exports a set of traces to a HTML file
///
/// # Errors
///
/// Can have errors if we fail to create a file or write to it
pub fn export_html_no_file_input(
    file_name_input: impl AsRef<Path> + Display,
    traces: Vec<(String, Vec<u128>)>,
) -> io::Result<usize> {
    let mut plot = Plot::new(); //make a new plotly plot
    for (name, trace) in traces {
        plot.add_trace(Histogram::new(trace).name(name)); //for each trace, add it to a plotly plot
    }

    let mut file = File::create(format!("{file_name_input}.html"))?; //make a file
    let html = plot.to_html(); //make the html
    let html = html.as_bytes(); //get the bytes - 2 steps to avoid dropping temporary value
    file.write_all(html)?; //write all of the bytes

    Ok(html.len())
}

#[derive(Copy, Clone, Debug, ValueEnum, strum::Display)]
#[allow(clippy::upper_case_acronyms)]
///Any format
pub enum ExportType {
    ///HTML graph
    HTML,
    ///CSV file with everything
    CSV,
}

impl ExportType {
    ///Export to the relevant format
    ///
    /// # Errors
    /// If we can't write to or create the file
    #[instrument]
    pub fn export(
        self,
        trace_name: String,
        runs: Vec<u128>,
        export_file_name: String,
    ) -> io::Result<usize> {
        match self {
            Self::HTML => export_html(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(), //since we don't have any extra traces for here, we just give it an empty list. If we don't give it a type using the turbofish, then we get compiler errors on interpreting generics.
            ),
            Self::CSV => export_csv(
                Some((trace_name, runs)),
                export_file_name,
                Vec::<String>::new(),
            ),
        }
    }
}
