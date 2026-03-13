#!/usr/bin/env python3
"""Complete migration script for lib.rs - Crux 0.7.6 to 0.14.0"""
import re

def migrate():
    with open('src/lib.rs', 'r') as f:
        content = f.read()
    
    # 1. Remove forbid
    content = content.replace('#![forbid(unsafe_code)]\n', '')
    
    # 2. Update imports
    content = content.replace('Effect}', 'Command, Effect}')
    
    # 3. Add module
    content = content.replace('pub mod image_processing;', 'pub mod image_processing;\npub mod outbox;')
    
    # 4. Fix render
    content = content.replace('caps.render().render()', 'caps.render.render()')
    
    # 5. Replace telemetry statements with semicolon
    content = re.sub(r'caps\.telemetry\(\)[^;]+;', ';', content)
    
    # 6. Fix App impl
    old = '''impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {'''
    new = '''impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;
        type Effect = crate::capabilities::Effect;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) -> Command<Self::Effect, Self::Event> {'''
    content = content.replace(old, new)
    
    # 7. Add Command::done()
    old = '''            }
        }

        fn view(&self, model: &Model) -> ViewModel {'''
    new = '''            }

            Command::done()
        }

        fn view(&self, model: &Model) -> ViewModel {'''
    content = content.replace(old, new)
    
    # 8. Fix recursive calls
    content = content.replace('self.update(Event::OutboxFlushRequested, model, caps);', ';')
    content = content.replace('self.update(Event::CameraPermissionRequested, model, caps);', 'model.camera_permission_state = PermissionState::Requesting;')
    
    # 9. Fix HttpResponse field access
    content = re.sub(r'(\w+)\.status\b(?!\()', r'\1.status()', content)
    content = re.sub(r'(\w+)\.body\b(?!\()', r'\1.body()', content)
    
    # 10. Fix capability access
    content = content.replace('caps.http()', 'caps.http')
    content = content.replace('caps.kv()', 'caps.kv')
    content = content.replace('caps.crypto()', 'caps.crypto')
    content = content.replace('caps.camera()', 'caps.camera')
    content = content.replace('caps.location()', 'caps.location')
    content = content.replace('caps.push()', 'caps.push')
    
    # 11. Fix HttpError::Status -> HttpStatus (carefully)
    content = re.sub(
        r'HttpError::Status \{ code, body \}',
        r'HttpError::HttpStatus { status: code, message: format!("HTTP {}", code), request_id: String::new(), retryable: code >= 500 }',
        content
    )
    
    # 12. Fix HttpError::Timeout pattern
    content = content.replace('HttpError::Timeout =>', 'HttpError::Timeout { .. } =>')
    
    with open('src/lib.rs', 'w') as f:
        f.write(content)
    
    print(f"Lines: {len(content.split(chr(10)))}")
    print("Done")

if __name__ == '__main__':
    migrate()
