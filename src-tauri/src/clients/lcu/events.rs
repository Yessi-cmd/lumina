use super::models::LcuEvent;

type Handler = Box<dyn Fn(&LcuEvent) + Send + Sync>;

/// Dispatches LCU WebSocket events to handlers registered by URI.
#[derive(Default)]
pub struct UriRouter {
    routes: Vec<(&'static str, Handler)>,
}

impl UriRouter {
    pub fn on<F>(mut self, uri: &'static str, handler: F) -> Self
    where
        F: Fn(&LcuEvent) + Send + Sync + 'static,
    {
        self.routes.push((uri, Box::new(handler)));
        self
    }

    pub fn dispatch(&self, event: &LcuEvent) {
        for (uri, handler) in &self.routes {
            if *uri == event.uri {
                handler(event);
            }
        }
    }
}
