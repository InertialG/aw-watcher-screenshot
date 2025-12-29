use tokio::sync::mpsc::{Sender, Receiver};

use crate::capture::event::MonitorImageEvent;

pub struct Storage {
    image_incoming: Receiver<MonitorImageEvent>,
}

impl Storage {


    pub fn run() {

    }
}
