use anyhow::{anyhow, Error};
use log::{log, Level};
use serde::{Deserialize, Serialize};
use tokio::net::unix::pipe::{OpenOptions, Receiver, Sender};

pub mod api;
pub mod command;

/**
 * Module for safely getting environment variables, logging any errors that occur and providing
 * default values.
 */
pub mod env {
    use std::env;
    use log::{Level, log};

    /**
     * Get the path to the devcade directory. This is where games are installed.
     * If the value is not set in the environment, it will default to /tmp/devcade.
     */
    pub fn devcade_path() -> String {
        let path = env::var("DEVCADE_PATH");

        match path {
            Ok(path) => path,
            Err(e) => {
                log!(Level::Warn, "Error getting DEVCADE_PATH falling back to '/tmp/devcade': {}", e);
                env::set_var("DEVCADE_PATH", "/tmp/devcade");
                String::from("/tmp/devcade")
            }
        }
    }

    /**
     * Get the URL of the API. This is where games are downloaded from.
     * If the value is not set in the environment, it will throw a fatal error and panic.
     */
    pub fn api_url() -> String {
        let url = env::var("DEVCADE_API_URL");

        match url {
            Ok(url) => url,
            Err(e) => {
                log!(Level::Error, "Error getting DEVCADE_API_URL: {}", e);
                panic!();
            }
        }
    }
}

/**
 * A game from the Devcade API
 */
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DevcadeGame {
    pub id: String,
    pub author: String,
    pub upload_date: String,
    pub name: String,
    pub hash: String,
    pub description: String,
    pub icon_link: String,
    pub banner_link: String,
}

impl Default for DevcadeGame {
    fn default() -> Self {
        DevcadeGame {
            id: String::new(),
            author: String::new(),
            upload_date: String::from("1970-01-01T00:00:00.000Z"),
            name: String::new(),
            hash: String::new(),
            description: String::new(),
            icon_link: String::new(),
            banner_link: String::new(),
        }
    }
}


/**
 * Make a FIFO at the given path. Uses an unsafe call to libc::mkfifo.
 */
fn mkfifo(path: &str) -> Result<(), Error> {
    log!(Level::Info, "Creating FIFO at {}", path);
    let path = std::path::Path::new(path);
    if path.exists() {
        // TODO: Check if it's a FIFO
        return Ok(());
    }
    if !path.parent().unwrap().exists() {
        std::fs::create_dir_all(path.parent().expect("Path has no parent"))?;
    }
    let path = path.to_str().expect("Path is not valid UTF-8");
    let path = std::ffi::CString::new(path)?;
    unsafe {
        let exit_code = libc::mkfifo(path.as_ptr(), 0o644);
        match exit_code {
            0 => Ok(()),
            c => Err(anyhow!(format!("mkfifo exited with code {}", c))),
        }
    }
}

/**
 * Opens a FIFO for reading. If the FIFO does not exist, it will be created.
 */
pub fn open_read_pipe(path: &str) -> Result<Receiver, Error> {
    if !std::path::Path::new(path).exists() {
        match mkfifo(path) {
            Ok(_) => (),
            Err(e) => {
                log!(Level::Error, "Error creating FIFO: {}", e);
                panic!();
            }
        }
    }

    let pipe = OpenOptions::new()
        // opening read_write allows the write end of the pipe to close without causing the read
        // end to close as well. This is necessary as if there is an unexpected error in the onboard
        // this will allow the main process to continue and wait for the onboard to restart.
        .read_write(true)
        .open_receiver(path)?;

    Ok(pipe)
}

/**
 * Opens a FIFO for writing. If the FIFO does not exist, it will be created.
 */
pub fn open_write_pipe(path: &str) -> Result<Sender, Error> {
    if !std::path::Path::new(path).exists() {
        match mkfifo(path) {
            Ok(_) => (),
            Err(e) => {
                log!(Level::Error, "Error creating FIFO: {}", e);
                panic!();
            }
        }
    }

    let pipe = OpenOptions::new()
        .open_sender(path)?;

    Ok(pipe)
}