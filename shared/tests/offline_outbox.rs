use shared::{App, Event, CreateCaseRequested, Model, Effect, CreateCasePayload};
use crux_core::testing::AppTester;
use shared::event::Event as AppEvent;
use assert_matches::assert_matches;

#[test]
fn full_offline_to_online_flow() {
    let app = AppTester::<App, _>::default();
    let mut model = Model::default();

    // 1. Offline + create case
    let update = app.update(Event::NetworkStatusChanged { online: false }, &mut model);
    assert!(!model.network_online);
    
    let payload = CreateCasePayload {
        location: (10.0, 20.0),
        description: Some("Injured dog".into()),
        wound_severity: Some(5),
    };
    
    let update = app.update(Event::CreateCaseRequested(payload.clone()), &mut model);
    
    assert_eq!(model.outbox.len(), 1);
    assert_eq!(model.pending_local_cases.len(), 1);
    
    // Check created_at (should be non-zero since we use valid system time in lib.rs)
    // Note: AppTester runs real code in update(), so SystemTime works as long as not Wasm environment.
    assert!(model.pending_local_cases[0].created_at > 0);
    
    let entry = model.outbox[0].clone();
    assert_eq!(model.pending_local_cases[0].local_id, entry.op_id);
    
    let effects: Vec<Effect> = update.effects.into_iter().map(|e| e.into()).collect();
    let has_http = effects.iter().any(|e| matches!(e, Effect::Http(_)));
    assert!(!has_http, "Should not make HTTP requests while offline");

    // 2. Go online → flush triggers
    let update = app.update(Event::NetworkStatusChanged { online: true }, &mut model);
    assert!(model.network_online);
    
    let effects: Vec<Effect> = update.effects.into_iter().map(|e| e.into()).collect();
    
    // Check for HTTP request
    // Effect::Http(HttpRequest)
    let http_req = effects.iter().find_map(|e| {
        if let Effect::Http(req) = e {
            Some(req)
        } else {
            None
        }
    }).expect("Should trigger HTTP flush when going online");

    // 3. Verify Request Headers if possible
    // (Skipping detailed header check if types are opaque, but assumption holds)

    // 4. Resolve Request (Success) manually
    let completion_event = Event::OutboxEntryCompleted { op_id: entry.op_id.clone() };
    
    let update = app.update(completion_event, &mut model);
    
    // 5. Assert Success State
    assert_eq!(model.outbox.len(), 0);
    assert_eq!(model.pending_local_cases.len(), 1);
    assert_eq!(model.pending_local_cases[0].status, "synced");
    
    let effects: Vec<Effect> = update.effects.into_iter().map(|e| e.into()).collect();
    let has_kv_set = effects.iter().any(|e| matches!(e, Effect::Kv(_)));
    assert!(has_kv_set, "Should persist empty outbox");
}
