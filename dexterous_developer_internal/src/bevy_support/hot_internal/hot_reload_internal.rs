use bevy::{prelude::*, window::PrimaryWindow};

pub use crate::hot_internal::hot_reload_internal::InternalHotReload;
use crate::{ReloadMode, ReloadSettings};

impl Resource for InternalHotReload {}

pub fn draw_internal_hot_reload(
    internal: Res<InternalHotReload>,
    settings: Option<Res<ReloadSettings>>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Some(settings) = settings else {
        return;
    };
    if (!settings.display_update_time
        && matches!(settings.reload_mode, ReloadMode::Full)
        && settings.reloadable_element_selection.is_none())
        || !(internal.is_changed() || settings.is_changed())
    {
        return;
    }

    let reload_mode = settings.reload_mode;
    let reloadable_element_selection = settings
        .reloadable_element_selection
        .unwrap_or("all reloadables")
        .trim_end_matches("_dexterous_developerd_inner_reloadable");

    let update = internal
        .last_update_date_time
        .format("%H:%M:%S")
        .to_string();

    let update = match reload_mode {
        crate::ReloadMode::Full => {
            format!("{update} - Full Update - {reloadable_element_selection}")
        }
        crate::ReloadMode::SystemAndSetup => {
            format!("{update} - Systems and Setup Functions - {reloadable_element_selection}")
        }
        crate::ReloadMode::SystemOnly => {
            format!("{update} - Systems Only - {reloadable_element_selection}")
        }
    };

    for mut window in &mut window {
        let title = window.title.split("::").next().unwrap_or("").trim();

        window.title = format!("{title} :: Updated {update}");
    }
}
