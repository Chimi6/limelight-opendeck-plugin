use openaction::{Instance, OpenActionResult};

use crate::actions::define_action;
use crate::poller;

define_action!(Battery, "battery", {
    async fn key_up(
        &self,
        _instance: &Instance,
        _settings: &Self::Settings,
    ) -> OpenActionResult<()> {
        poller::request_poll();
        Ok(())
    }
});
