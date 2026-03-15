use shared::{App, CruxApp, Event, Model, UserProfile};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, Listener};
use tauri_plugin_nativemap;
use serde_json::Value as JsonValue;

/// Global state: holds the Crux App instance and current model
pub struct CruxState {
    pub app: App,
    pub model: Mutex<Model>,
}

impl CruxState {
    pub fn new() -> Self {
        Self {
            app: App::default(),
            model: Mutex::new(Model::default()),
        }
    }
}

/// Convert UI event string to Crux Event enum
fn string_to_event(event: &str, payload: Option<&JsonValue>) -> Event {
    match event {
        "AppStarted" => Event::AppStarted,
        "OnboardingComplete" => Event::OnboardingComplete,
        "LoginRequested" => Event::LoginRequested,
        "LoginCompleted" => {
            if let Some(p) = payload {
                let jwt = p.get("jwt").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let user_id = p.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let user_type = p.get("metadata")
                    .and_then(|m| m.get("user_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("individual")
                    .to_string();
                Event::LoginCompleted { jwt, user_id, user_type }
            } else {
                Event::Noop
            }
        }
        "LogoutRequested" => Event::LogoutRequested,
        "SwitchToMap" => Event::SwitchToMap,
        "SwitchToList" => Event::SwitchToList,
        "ReportSpotted" => Event::CapturePhotoRequested,
        "CaseSelected" => {
            let id = payload
                .and_then(|p| p.get("case_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Event::CaseSelected { case_id: id }
        }
        "DismissError" => Event::DismissError,
        "MapReady" => Event::MapReady,
        "MapPinTapped" => {
            let id = payload
                .and_then(|p| p.as_str())
                .unwrap_or("unknown")
                .to_string();
            Event::MapPinTapped { case_id: id }
        }
        "CommunityRequested" => Event::CommunityRequested,
        "MessageMemberRequested" => {
            let id = payload
                .and_then(|p| p.get("member_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            Event::MessageMemberRequested { member_id: id }
        }
        "ChatClosed" => Event::ChatClosed,
        "CaseDeselected" => Event::CaseDeselected,
        "ProfileUpdated" => {
            if let Some(p) = payload {
                match serde_json::from_value(p.clone()) {
                    Ok(profile) => Event::ProfileUpdated(profile),
                    Err(e) => {
                        tracing::error!("❌ Failed to deserialize profile: {}", e);
                        Event::Noop
                    }
                }
            } else {
                Event::Noop
            }
        }
        "MfaRequired" => {
            let challenge_id = payload
                .and_then(|p| p.get("challenge_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Event::MfaRequired { challenge_id }
        }
        "MfaVerifyRequested" => {
            let code = payload
                .and_then(|p| p.get("code"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Event::MfaVerifyRequested { code }
        }
        _ => Event::Noop,
    }
}

/// Emit ViewModel update to UI
fn emit_view_model(app_handle: &AppHandle, state: &State<'_, CruxState>) -> Result<(), String> {
    let model = state.model.lock().map_err(|e| e.to_string())?;
    let view_model = state.app.view(&model);
    drop(model);
    
    app_handle.emit("crux-update", view_model).map_err(|e| e.to_string())?;
    tracing::debug!("✅ Emitted ViewModel");
    Ok(())
}

/// Tauri command: Dispatch event from UI to Crux
#[tauri::command]
fn handle_event(
    app_handle: AppHandle,
    state: State<'_, CruxState>,
    event: String,
    payload: Option<JsonValue>,
) -> Result<(), String> {
    tracing::info!("📥 Event: {} payload: {:?}", event, payload);

    let crux_event = string_to_event(&event, payload.as_ref());
    let mut model = state.model.lock().map_err(|e| e.to_string())?;
    let _commands = state.app.update(crux_event, &mut model, &shared::capabilities::Capabilities::default());
    drop(model);
    
    emit_view_model(&app_handle, &state)?;
    Ok(())
}

/// Initialize and run the Tauri application
pub fn run_app() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
    tracing::info!("🚀 Starting WarmStreet");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_nativemap::init())
        .manage(CruxState::new())
        .invoke_handler(tauri::generate_handler![handle_event])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Listen for "map-pin-tapped" events emitted by the Swift/Kotlin layer.
            // Forward them into Crux as a MapPinTapped event.
            let state: tauri::State<'_, CruxState> = app.state();
            let _state_ref = state.inner();

            app_handle.clone().listen("map-pin-tapped", move |tauri_event| {
                let payload = tauri_event.payload();
                let case_id = payload.trim_matches('"').to_string();
                tracing::info!("📍 map-pin-tapped: caseId={}", case_id);
                // Re-dispatch as a Crux event via the existing handle_event pathway.
                // We fire it as a shell-side event directly on the app handle.
                app_handle.emit("crux-map-pin-tapped", case_id).ok();
            });

            // Listen for "map-ready" to tell Crux the native view is up.
            let app_handle2 = app.handle().clone();
            app_handle2.clone().listen("map-ready", move |_| {
                tracing::info!("🗺️ map-ready received — forwarding MapReady to Crux");
                // The Crux core needs a MapReady event — the frontend JS will dispatch it.
                app_handle2.emit("crux-map-ready", ()).ok();
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running WarmStreet");
}
