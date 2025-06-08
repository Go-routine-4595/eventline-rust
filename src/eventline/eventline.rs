use super::ui;

use std::{
    cell::RefCell,
    time::Duration,
};

use color_eyre::Result;
use crossterm::event::{KeyCode, poll, Event as CEvent};
use ratatui::{
    DefaultTerminal,
};

use tokio::sync::mpsc::{
    unbounded_channel,
    UnboundedReceiver,
    UnboundedSender,
    error::TryRecvError,
};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

// Constants for sorting order
pub const ASC: i32 = 0;
pub const DESC: i32 = 1;

pub trait Event {
    fn get_event_presentation(&self) -> String;
    fn get_event_time(&self) -> chrono::DateTime<chrono::Utc>;
    fn get_event_id(&self) -> String;
    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_dyn(&self) -> Box<dyn Event + Send>;
}

pub struct Events<E: ?Sized = dyn Event> {
    pub global_counter: i32,
    pub last_update: String,
    pub events_map: std::collections::HashMap<String, Box<E>>
}

impl<E: ?Sized + Clone> Clone for Events<E>
where
    Box<E>: Clone,
{
    fn clone(&self) -> Self {
        Events {
            global_counter: self.global_counter,
            last_update: self.last_update.clone(),
            events_map: self.events_map.clone(),
        }
    }
}

pub struct EventLine {
    event_receiver: Option<UnboundedReceiver<Box<Events<dyn Event + Send>>>>,
    events_data: Option<Events<dyn Event + Send>>,
    title: String,
    data_list: Vec<String>,
    term: RefCell<DefaultTerminal>,
    ui_handler: ui::UI,
    shutdown_tx: Option<oneshot::Sender<bool>>,
}

impl EventLine {
    fn check_for_keypress(&self) -> Result<Option<KeyCode>> {
        if poll(std::time::Duration::from_millis(100))? {
            if let CEvent::Key(key) = crossterm::event::read()? {
                return Ok(Some(key.code));
            }
        }
        Ok(None)
    }

    pub fn new(s: String) -> Result<Self, String> {
        match color_eyre::install() {
            Ok(_) => {},
            Err(e) => return Err(format!("error: {}", e)),
        }

        Ok(EventLine {
            event_receiver: None,
            events_data: None,
            title: s,
            data_list: Vec::new(),
            term: RefCell::new(ratatui::init()),
            ui_handler: ui::UI::new(),
            shutdown_tx: None,
        })
    }

    pub fn with_shutdown(&mut self, shutdown_tx: oneshot::Sender<bool>) -> &Self {
        self.shutdown_tx = Some(shutdown_tx);
        self
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn data_list(&self) -> &Vec<String> {
        &self.data_list
    }

    pub fn global_counter(&self) -> i32 {
        if let Some(events) = self.events_data.as_ref() {
            return events.global_counter;
        }
        return 0
    }
    pub fn last_update(&self) -> String {
        if let Some(events) = self.events_data.as_ref() {
            return events.last_update.clone();
        }
        return "".to_string();
    }

    pub fn events_map_size(&self) -> usize {
        if self.events_data.is_none() {
            return 0;
        }
        self.events_data.as_ref().unwrap().events_map.len()
    }
    // Create a channel for event communication and return the sender

    // Alternative method that creates a bounded channel with specified capacity
    pub fn create_event_channel(&mut self) -> UnboundedSender<Box<Events<dyn Event + Send>>> {
        // Using crossbeam's bounded channel
        let (sender, receiver) = unbounded_channel();
        self.event_receiver = Some(receiver);
        sender
    }

    // Start the event processing in a thread and return a join handle
    pub fn start_in_thread(mut self) -> JoinHandle<()> {
        let handle = tokio::spawn(async move {
            self.start().await
        });
        handle
    }

    pub async fn start(&mut self) -> () {
        //let mut terminal: DefaultTerminal = ratatui::init();

        println!("EventLine started");
        self.ui_handler.render(self);
        loop {
            // Check for keyboard events without causing errors
            match ui::process_keypress() {
                Ok(exit) => {
                    if exit {
                        ratatui::restore();
                        if let Some(tx) = self.shutdown_tx.take() {
                            tx.send(true).unwrap_or_else(|_| println!("Error sending shutdown signal"))
                        }
                        return;
                        //return Ok(());
                    }
                }
                Err(_) => {}
            }

            // Check for any events received through the channel
            match &mut self.event_receiver {
                Some(receiver) => {
                    tokio::select! {
                        maybe_events = receiver.recv() => {
                            match maybe_events {
                                Some(events) => {
                                    // Process the received event
                                    //self.data_list = EventLine::sort_map_by_time(&events, ASC, events.last_update.clone()).0;
                                    self.data_list = EventLine::sort_map_by_key(&events, events.last_update.clone()).0;
                                    self.events_data = Some(*events);
                                },
                                None => {
                                    // Channel is closed
                                }
                            }
                        }
                        // You can add other async operations here to select between them
                        // For example:
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {
                            // Timeout occurred
                        }
                    }
                }
                None => {} // Handle the case when event_receiver is None
            }

            //terminal.draw(|f| ui::render(f, self));
            //self.term.borrow_mut().draw(|f| ui::render(f, self));
            self.ui_handler.render(self)
        }
    }

    fn apply_style(d: &Events<dyn Event + Send>) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for (_, event) in d.events_map.iter() {
            v.push(event.get_event_presentation());
        }
        v
    }

    /// Sorts a HashMap by its string keys in ascending order and returns a vector of event presentations.
    /// Also returns the index of the event with the key that matches last_update.
    fn sort_map_by_key(data: &Events<dyn Event + Send>, last_update: String) -> (Vec<String>, usize)
    {
        let mut last_index_update = 0;

        // Get all keys from the map
        let mut keys: Vec<String> = data.events_map.keys().cloned().collect();

        // Sort the keys
        keys.sort();

        // Build the ordered result
        let mut result = Vec::with_capacity(data.events_map.len());

        for (i, key) in keys.iter().enumerate() {
            if let Some(event) = data.events_map.get(key) {
                result.push(event.get_event_presentation());
                if last_update == *key {
                    last_index_update = i;
                }
            }
        }

        (result, last_index_update)
    }


    /// Sorts a HashMap of events by timestamp in ascending or descending order.
    /// Returns a vector of event presentations in the sorted order and the index of the event with the specified ID.
    pub fn sort_map_by_time(
        data: &Events<dyn Event + Send>,
        order: i32,
        last_update: String
    ) -> (Vec<String>, usize) {
        let count = data.events_map.len();
        // we know the capacity we want to re-organize the way the element are ordered
        let mut result = vec![String::new(); count];
        let mut timestamps = vec![0; count];
        let mut ids = vec![String::new(); count];

        let mut index = 0;

        for (key, event) in &data.events_map {
            let mut pos_to_insert = index;
            let v = event.get_event_time().timestamp_millis();

            let _debug_time = event.get_event_time();
            let _debug = event.get_event_presentation();

            // Find position to insert based on the timestamp ordering
            for i in 0..index {
                if EventLine::compare_using(v, timestamps[i], order) {
                    // If condition is true, we shift elements down and insert at position i
                    EventLine::shift_down_from_index(&mut timestamps, &mut result, &mut ids, index, i);
                    pos_to_insert = i;
                    break;
                }
            }

            timestamps[pos_to_insert] = v;
            result[pos_to_insert] = event.get_event_presentation();
            ids[pos_to_insert] = key.clone();
            index += 1;
        }

        // Find the index of the last_update event
        for (i, id) in ids.iter().enumerate() {
            if id == &last_update {
                return (result, i);
            }
        }

        (result, 0)
    }

    /// Shifts elements in three vectors down from start index to stop index.
    fn shift_down_from_index(
        timestamps: &mut Vec<i64>,
        presentations: &mut Vec<String>,
        ids: &mut Vec<String>,
        start: usize,
        stop: usize
    ) {
        // No need to resize the vector there are declared as a fixed size slice we know
        // exactly the capacity we need.

        // Shift elements down
        for i in (stop + 1..=start).rev() {
            timestamps[i] = timestamps[i - 1];
            presentations[i] = presentations[i - 1].clone();
            ids[i] = ids[i - 1].clone();
        }
    }

    /// Compares two i64 values based on the specified order.
    fn compare_using(a: i64, b: i64, order: i32) -> bool {
        match order {
            1 => a >= b,
            0 => a < b,
            _ => false,
        }
    }
}
