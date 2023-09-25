use bevy::{prelude::*, window::PrimaryWindow};

pub use crate::hot_internal::hot_reload_internal::InternalHotReload;
use crate::ReloadSettings;

impl Resource for InternalHotReload {}

pub fn draw_internal_hot_reload(
    internal: Res<InternalHotReload>,
    settings: Option<Res<ReloadSettings>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(settings) = settings else {
        return;
    };
    if !settings.display_update_time || !internal.is_changed() {
        return;
    }

    let update = internal
        .last_update_date_time
        .format("%H:%M:%S")
        .to_string();

    for mut window in &mut window {
        let title = window.title.split("::").next().unwrap_or("").trim();

        window.title = format!("{title} :: Updated {update}");
    }
}
