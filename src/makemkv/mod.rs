use std::{path::Path, process::Stdio};
use csv::CsvRowIter;
use messaging::MakemkvMessage;
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{Child, ChildStdout, Command},
};

mod csv;
mod messaging;

pub struct Makemkv {
    command: Child,
    line_buffer: Lines<BufReader<ChildStdout>>,
}
impl Makemkv {
    pub fn rip(device: impl AsRef<str>, destination: &Path) -> std::io::Result<Self> {
        let device = device.as_ref();
        // -r --messages=-stdout --progress=-same --noscan mkv dev:/dev/sr2 all ./tmp/
        let mut command = Command::new("makemkvcon")
            .arg("-r")
            .arg("--messages=-stdout")
            .arg("--progress=-same")
            .arg("--noscan")
            .arg("mkv")
            .arg(format!("dev:{device}"))
            .arg("all")
            .arg(destination)
            .stdout(Stdio::piped())
            .spawn()?;

        let line_buffer = BufReader::new(command.stdout.take().unwrap()).lines();

        return Ok(Self {
            command,
            line_buffer,
        });
    }
    pub async fn next_event(&mut self) -> std::io::Result<Option<MakemkvMessage>> {
        loop {
            let line = self.line_buffer.next_line().await?;
            match line {
                Some(line) => {
                    let csv_cells = CsvRowIter::new(&line);
                    if let Some(message) = MakemkvMessage::from_iter(csv_cells) {
                        return Ok(Some(message));
                    }
                }
                None => {
                    return Ok(None);
                }
            }
        }
    }
}
