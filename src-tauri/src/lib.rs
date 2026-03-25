mod types;
mod image_processing;
mod crypto;
mod vision;

use serde_json::Value as JsonValue;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, Listener};
use tauri_plugin_nativemap;
use crate::types::{AppState as SharedAppState, Case, CaseStatus, FeedView, MapPin, UserProfile, ViewModel, CommunityMember, Coordinate};
use chrono::Utc;

/// Application state managed by Tauri
pub struct AppInstanceState {
    pub view_model: Mutex<ViewModel>,
    pub jwt: Mutex<Option<String>>,
    pub user_id: Mutex<Option<String>>,
}

impl AppInstanceState {
    pub fn new() -> Self {
        Self {
            view_model: Mutex::new(ViewModel::default()),
            jwt: Mutex::new(None),
            user_id: Mutex::new(None),
        }
    }
}

impl Default for AppInstanceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tauri command: Get current view model
#[tauri::command]
fn get_view_model(state: State<'_, AppInstanceState>) -> Result<ViewModel, String> {
    let view_model = state.view_model.lock().map_err(|e| e.to_string())?;
    Ok(view_model.clone())
}

/// Tauri command: Dispatch event from UI
#[tauri::command]
fn handle_event(
    app_handle: AppHandle,
    state: State<'_, AppInstanceState>,
    event: String,
    payload: Option<JsonValue>,
) -> Result<(), String> {
    tracing::info!("📥 Event: {} payload: {:?}", event, payload);

    let mut view_model = state.view_model.lock().map_err(|e| e.to_string())?;

    match event.as_str() {
        "AppStarted" => {
            view_model.status = SharedAppState::Unauthenticated;
            tracing::info!("✅ App started");
        }

        "OnboardingComplete" => {
            view_model.status = SharedAppState::Ready;
        }

        "LoginRequested" => {
            view_model.status = SharedAppState::Authenticating;
        }

        "LoginCompleted" => {
            if let Some(p) = &payload {
                let jwt = p.get("jwt").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let user_id = p.get("user_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

                *state.jwt.lock().map_err(|e| e.to_string())? = Some(jwt);
                *state.user_id.lock().map_err(|e| e.to_string())? = Some(user_id);

                view_model.status = SharedAppState::Ready;

                // Add some sample cases for testing
                view_model.cases = vec![
                    Case {
                        id: "1".to_string(),
                        location: Coordinate { lat: 37.7749, lon: -122.4194 },
                        description: "Dog with injured leg".to_string(),
                        status: CaseStatus::Pending,
                        severity: 3,
                        case_type: "Dog".to_string(),
                        age: Some("Adult".to_string()),
                        breed: Some("Mixed".to_string()),
                        image_url: Some("https://images.unsplash.com/photo-1541233349642-6e425fe6190e?w=400&q=80".to_string()),
                        created_at: Utc::now(),
                        reporter_id: None,
                        assigned_rescuer_id: None,
                    },
                    Case {
                        id: "2".to_string(),
                        location: Coordinate { lat: 37.7849, lon: -122.4094 },
                        description: "Cat stuck in tree".to_string(),
                        status: CaseStatus::Claimed,
                        severity: 2,
                        case_type: "Cat".to_string(),
                        age: Some("Young".to_string()),
                        breed: Some("Tabby".to_string()),
                        image_url: Some("https://images.unsplash.com/photo-1543852786-1cf6624b9987?w=400&q=80".to_string()),
                        created_at: Utc::now(),
                        reporter_id: None,
                        assigned_rescuer_id: None,
                    },
                ];

                // Create map pins from cases
                view_model.map_pins = view_model.cases.iter().map(|c| MapPin {
                    case_id: c.id.clone(),
                    lat: c.location.lat,
                    lon: c.location.lon,
                    severity: c.severity,
                    status: c.status,
                }).collect();
            }
        }

        "LogoutRequested" => {
            *state.jwt.lock().map_err(|e| e.to_string())? = None;
            *state.user_id.lock().map_err(|e| e.to_string())? = None;
            view_model.status = SharedAppState::Unauthenticated;
            view_model.cases.clear();
            view_model.map_pins.clear();
        }

        "SwitchToMap" => {
            view_model.feed_view = FeedView::Map;
        }

        "SwitchToList" => {
            view_model.feed_view = FeedView::List;
        }

        "CaseSelected" => {
            if let Some(p) = &payload {
                let case_id = p.get("case_id").and_then(|v| v.as_str()).unwrap_or("");
                view_model.selected_case = view_model.cases.iter()
                    .find(|c| c.id == case_id)
                    .cloned();
            }
        }

        "CaseDeselected" => {
            view_model.selected_case = None;
        }

        "ReportSpotted" => {
            view_model.toast = Some("Camera feature coming soon!".to_string());
        }

        "DismissError" => {
            view_model.error = None;
        }

        "MapReady" => {
            tracing::info!("🗺️ Map is ready");
        }

        "MapPinTapped" => {
            if let Some(p) = &payload {
                let case_id = p.as_str().unwrap_or("");
                tracing::info!("📍 Pin tapped: {}", case_id);
                view_model.selected_case = view_model.cases.iter()
                    .find(|c| c.id == case_id)
                    .cloned();
            }
        }

        "CommunityRequested" => {
            view_model.is_loading_community = true;
            // Simulate loading community members
            view_model.community_members = vec![
                CommunityMember {
                    id: "1".to_string(),
                    name: "Sarah Chen".to_string(),
                    member_type: "Individual".to_string(),
                    description: "Passionate rescuer with first aid kit.".to_string(),
                    location_name: "Central Park".to_string(),
                    phone: "+1234567890".to_string(),
                    image_url: "https://i.pravatar.cc/150?u=sarah".to_string(),
                    lat: 40.785091,
                    lon: -73.968285,
                    karma: 150,
                    last_active: "2m ago".to_string(),
                },
                CommunityMember {
                    id: "2".to_string(),
                    name: "Marcus Johnson".to_string(),
                    member_type: "Vet".to_string(),
                    description: "Licensed veterinarian, specializing in small animals.".to_string(),
                    location_name: "Uptown Clinic".to_string(),
                    phone: "+1234567891".to_string(),
                    image_url: "https://i.pravatar.cc/150?u=marcus".to_string(),
                    lat: 40.811550,
                    lon: -73.946477,
                    karma: 320,
                    last_active: "5m ago".to_string(),
                },
                CommunityMember {
                    id: "3".to_string(),
                    name: "David Martinez".to_string(),
                    member_type: "NGO".to_string(),
                    description: "Local NGO focusing on urban animal welfare.".to_string(),
                    location_name: "West Side Shelter".to_string(),
                    phone: "+1234567892".to_string(),
                    image_url: "https://i.pravatar.cc/150?u=david".to_string(),
                    lat: 40.771133,
                    lon: -73.984139,
                    karma: 580,
                    last_active: "10m ago".to_string(),
                },
            ];
            view_model.is_loading_community = false;
        }

        "MessageMemberRequested" => {
            if let Some(p) = &payload {
                let member_id = p.get("member_id").and_then(|v| v.as_str()).unwrap_or("");
                view_model.active_chat_member = view_model.community_members.iter()
                    .find(|m| m.id == member_id)
                    .cloned();
            }
        }

        "ChatClosed" => {
            view_model.active_chat_member = None;
        }

        "ProfileUpdated" => {
            if let Some(p) = &payload {
                match serde_json::from_value::<UserProfile>(p.clone()) {
                    Ok(profile) => {
                        view_model.profile = Some(profile);
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to deserialize profile: {}", e);
                    }
                }
            }
        }

        "MfaRequired" => {
            view_model.status = SharedAppState::MfaVerification;
        }

        "MfaVerifyRequested" => {
            // Handle MFA verification
            view_model.status = SharedAppState::Ready;
        }

        _ => {
            tracing::warn!("⚠️ Unknown event: {}", event);
        }
    }

    drop(view_model);

    // Emit updated view model to UI
    emit_view_model(&app_handle, &state)?;

    Ok(())
}

/// Emit ViewModel update to UI
fn emit_view_model(app_handle: &AppHandle, state: &State<'_, AppInstanceState>) -> Result<(), String> {
    let view_model = state.view_model.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    app_handle.emit("state-update", &*view_model).map_err(|e| e.to_string())?;
    tracing::debug!("✅ Emitted ViewModel");
    Ok(())
}

/// Initialize and run the Tauri application
pub fn run_app() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    tracing::info!("🚀 Starting WarmStreet");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_nativemap::init())
        .manage(AppInstanceState::new())
        .invoke_handler(tauri::generate_handler![
            handle_event,
            get_view_model
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Listen for "map-pin-tapped" events from native map layer
            let app_handle_clone = app_handle.clone();
            app_handle.listen("map-pin-tapped", move |tauri_event| {
                let payload = tauri_event.payload();
                let case_id = payload.trim_matches('"').to_string();
                tracing::info!("📍 map-pin-tapped: caseId={}", case_id);

                // Forward to handle_event
                let state: tauri::State<'_, AppInstanceState> = app_handle_clone.state();
                let _ = handle_event(
                    app_handle_clone.clone(),
                    state,
                    "MapPinTapped".to_string(),
                    Some(serde_json::json!(case_id)),
                );
            });

            // Listen for "map-ready" event
            let app_handle2 = app_handle.clone();
            app_handle.listen("map-ready", move |_| {
                tracing::info!("🗺️ map-ready received");
                let state: tauri::State<'_, AppInstanceState> = app_handle2.state();
                let _ = handle_event(
                    app_handle2.clone(),
                    state,
                    "MapReady".to_string(),
                    None,
                );
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running WarmStreet");
}
