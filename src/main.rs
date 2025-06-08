mod eventline;

use crate::eventline::eventline::{Event, EventLine, Events};
use std::thread;
use std::time::Duration;

use fakeit::company::company;
use fakeit::datetime::DateTime;
use fakeit::{address, company, datetime, unique};
#[tokio::main]

async fn main() -> Result<(), String> {
    //let log_events = log()?;

    match EventLine::new(String::from("my title")) {
        Ok(mut ev) => {
            // ev.test(vec!["1".to_string(),"2".to_string(),"3".to_string(),"4".to_string(),"5".to_string(),"6".to_string(),"7".to_string(),"8".to_string(),"9".to_string(),"10".to_string()]);
            // Create event channel using crossbeam
            let event_sender = ev.create_event_channel();

            // Start the EventLine in a thread
            let handle = ev.start_in_thread();
            //    .map_err(|e| format!("Failed to start thread: {}", e))?;

            let mut log_events = Events::<LogEvent> {
                global_counter: 0,
                last_update: "today".to_string(),
                events_map: std::collections::HashMap::new(),
            };

            let mut log_events_test = Events::<dyn Event + Send> {
                global_counter: 0,
                last_update: "today".to_string(),
                events_map: std::collections::HashMap::new(),
            };

            for i in 0..10 {
                let log_key = format!("log{}", i);
                let new_logged_event = LogEvent::new(format!("log message {}", i))?;
                log_events.last_update = new_logged_event.id.clone();
                log_events.global_counter += 1;
                log_events.events_map.insert(
                    log_key.clone(),
                    Box::new(LogEvent::new(format!("log message {}", i))?),
                );

                // alternative
                let new_logged_event_alt = LogEvent::new(format!("log message {}", i))?;
                log_events_test.last_update = new_logged_event_alt.id.clone();
                log_events_test.global_counter += 1;
                log_events_test.events_map.insert(
                    log_key.clone(),
                    Box::new(LogEvent::new(format!("log alt message {}", i))?),
                );

                let cloned_events = log_events.clone();
                // Create a new Events object with the expected type
                let mut boxed_events = Events {
                    global_counter: cloned_events.global_counter,
                    last_update: cloned_events.last_update,
                    events_map: std::collections::HashMap::new(),
                };

                // Convert each LogEvent to a boxed dyn Event
                for (key, log_event) in cloned_events.events_map {
                    boxed_events
                        .events_map
                        .insert(key, log_event as Box<dyn Event + Send>);
                }
                tokio::select! {
                    // Send the boxed events - use async block to make it a future
                    _ = async {
                        let _ = event_sender.send(Box::new(boxed_events));
                        println!("Sent events");
                        Ok::<_, std::io::Error>(())
                    } => {
                        // sending data
                    }
                    _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                        //println!("Sent events");
                    }
                }
                // Alternative approach: just send outside the select
                // tokio::select! {
                //     _ = tokio::time::sleep(Duration::from_millis(1000)) => {
                //         //println!("Sent events");
                //     }
                // }
                // let _ = event_sender.send(Box::new(boxed_events));
            }
            println!("Waiting for task to complete...");
            match handle.await {
                Ok(_) => println!("Task completed successfully"),
                Err(e) => println!("Task failed with error: {}", e),
            }
        }
        Err(e) => return Err(e.as_str().to_string()),
    }
    Ok(())
}

struct LogEvent {
    timestamp: chrono::DateTime<chrono::Utc>,
    message: String,
    id: String,
    company: String,
    country: String,
    city: String,
    date: DateTime,
}

impl Event for LogEvent {
    fn get_event_presentation(&self) -> String {
        //format!("[{}] {}", self.timestamp, self.message)
        format!(
            "Event: {:<20} - Compnay: {:<25} - country: {:<25} - City: {:<20} - time: {:<29}",
            self.id,
            self.company,
            self.country,
            self.city,
            chrono::DateTime::from_timestamp(self.date.secs, self.date.nsecs)
                .unwrap()
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%dT%H:%M:%S%:z")
        )
        //self.timestamp.with_timezone(&chrono::Local).format("%Y-%m-%dT%H:%M:%S%:z"))
    }

    fn get_event_time(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    fn get_event_id(&self) -> String {
        self.id.clone()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_dyn(&self) -> Box<dyn Event + Send> {
        Box::new(self.clone())
    }
}

impl LogEvent {
    fn new(message: String) -> Result<LogEvent, String> {
        let to_convert_timestamp = datetime::date();
        Ok(LogEvent {
            timestamp: chrono::DateTime::from_timestamp(
                to_convert_timestamp.secs,
                to_convert_timestamp.nsecs,
            )
            .unwrap(),
            message: message,
            id: fakeit::unique::uuid_v4(),
            company: company(),
            country: fakeit::address::country(),
            city: address::city(),
            date: to_convert_timestamp,
        })
    }
}

// Implement Clone for LogEvent
impl Clone for LogEvent {
    fn clone(&self) -> Self {
        LogEvent {
            timestamp: self.timestamp,
            message: self.message.clone(),
            id: self.id.clone(),
            company: self.company.clone(),
            country: self.country.clone(),
            city: self.city.clone(),
            date: fakeit::datetime::DateTime {
                secs: self.date.secs,
                nsecs: self.date.nsecs,
            },
        }
    }
}

impl Events<dyn Event + Send> {
    pub fn clone_events(&self) -> Self {
        let mut new_map = std::collections::HashMap::new();
        for (key, event) in &self.events_map {
            // We need to make a deep copy of the event
            // This requires that each concrete Event type implements Clone
            // or provides a way to duplicate itself

            // Clone the key and dereference the Box to get access to the underlying Event
            // Then create a new Box containing a clone or new instance of the event
            if let Some(_concrete_event) = self.clone_event(event.as_ref()) {
                new_map.insert(key.clone(), event.as_ref().clone_dyn());
            }
        }

        Events {
            global_counter: self.global_counter,
            last_update: self.last_update.clone(),
            events_map: new_map,
        }
    }

    // Helper method to create clones of specific event types
    fn clone_event(&self, event: &(dyn Event + Send)) -> Option<Box<dyn Event + Send>> {
        // Use the as_any method to try downcasting to specific event types
        // Then clone that specific type

        // Example (you need to implement this for your specific event types):
        if let Some(log_event) = event.as_any().downcast_ref::<LogEvent>() {
            return Some(Box::new(log_event.clone()));
        }

        // For now, this is a placeholder - you must implement this for your specific event types
        None
    }
}
