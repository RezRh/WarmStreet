use shared::{Effect, Model, AppState, Event};
use crux_core::testing::AppTester;

#[test]
fn test_onboarding_flow() {
    // In 0.10, AppTester typically takes <App, Effect>
    // Since we have the workaround Event: App, we use Event as the first param if needed, 
    // but let's try App first.
    let app = AppTester::<shared::App, Effect>::default();
    let mut model = Model::default();

    assert_eq!(model.state, AppState::Unauthenticated);

    // 2. LoginRequested
    let update = app.update(Event::LoginRequested, &mut model);
    assert_eq!(model.state, AppState::Authenticating);
    assert!(update.effects.iter().any(|e| matches!(e, Effect::Render(_))));

    // 3. LoginCompleted
    let update = app.update(Event::LoginCompleted { 
        jwt: "xxx".to_string(), 
        user_id: "123".to_string() 
    }, &mut model);
    assert_eq!(model.state, AppState::OnboardingLocation);
    assert_eq!(model.jwt, Some("xxx".to_string()));
    assert_eq!(model.user_id, Some("123".to_string()));
    assert!(update.effects.iter().any(|e| matches!(e, Effect::Render(_))));

    // 4. LocationPinDropped
    let update = app.update(Event::LocationPinDropped { 
        lat: 37.7749, 
        lng: -122.4194 
    }, &mut model);
    assert_eq!(model.state, AppState::OnboardingRadius);
    assert_eq!(model.area_center, Some((37.7749, -122.4194)));
    assert!(update.effects.iter().any(|e| matches!(e, Effect::Render(_))));

    // 5. RadiusSelected
    let update = app.update(Event::RadiusSelected { meters: 8000 }, &mut model);
    assert_eq!(model.state, AppState::Ready);
    assert_eq!(model.area_radius_m, 8000);
    assert!(update.effects.iter().any(|e| matches!(e, Effect::Render(_))));

    // 6. OnboardingComplete
    let update = app.update(Event::OnboardingComplete, &mut model);
    assert_eq!(model.state, AppState::Ready);
    assert!(update.effects.iter().any(|e| matches!(e, Effect::Render(_))));
}
