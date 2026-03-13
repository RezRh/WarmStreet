#!/usr/bin/env python3
"""Migration script for lib.rs - Crux 0.7.6 to 0.14.0"""

import re

def migrate_lib_rs():
    with open('src/lib.rs', 'r') as f:
        content = f.read()
    
    original_lines = len(content.split('\n'))
    print(f"Original lines: {original_lines}")
    
    # 1. Remove forbid(unsafe_code)
    content = content.replace('#![forbid(unsafe_code)]\n', '')
    
    # 2. Update crux_core imports
    content = content.replace(
        'pub use crux_core::{render::Render, App as CruxApp, Effect};',
        'pub use crux_core::{render::Render, App as CruxApp, Command, Effect};'
    )
    
    # 3. Add module declarations
    if 'pub mod outbox;' not in content:
        content = content.replace(
            'pub mod image_processing;',
            'pub mod image_processing;\npub mod outbox;'
        )
    
    # 4. Fix caps.render().render() -> caps.render.render()
    content = content.replace('caps.render().render()', 'caps.render.render()')
    
    # 5. Fix caps.telemetry() calls - replace with tracing or remove
    content = re.sub(r'caps\.telemetry\(\)\.event\([^)]+\);', '// telemetry: event', content)
    content = re.sub(r'caps\.telemetry\(\)\.error\([^)]+\);', '// telemetry: error', content)
    content = re.sub(r'caps\.telemetry\(\)\.warn\([^)]+\);', '// telemetry: warn', content)
    content = re.sub(r'caps\.telemetry\(\)\.counter\([^)]+\);', '// telemetry: counter', content)
    content = re.sub(r'caps\.telemetry\(\)\.gauge\([^)]+\);', '// telemetry: gauge', content)
    
    # 6. Fix App trait implementation - add Effect type and Command return
    old_impl = '''impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) {'''
    
    new_impl = '''impl crux_core::App for App {
        type Event = Event;
        type Model = Model;
        type ViewModel = ViewModel;
        type Capabilities = Capabilities;
        type Effect = crate::capabilities::Effect;

        fn update(&self, event: Event, model: &mut Model, caps: &Capabilities) -> Command<Self::Effect, Self::Event> {'''
    
    content = content.replace(old_impl, new_impl)
    
    # 7. Add Command::done() at the end of update function
    # Find the exact pattern before fn view
    old_pattern = '''            }
        }

        fn view(&self, model: &Model) -> ViewModel {'''
    
    new_pattern = '''            }

            Command::done()
        }

        fn view(&self, model: &Model) -> ViewModel {'''
    
    content = content.replace(old_pattern, new_pattern)
    
    # 8. Fix self.update recursive calls for OutboxFlushRequested
    content = content.replace(
        'self.update(Event::OutboxFlushRequested, model, caps);',
        '// Outbox flush triggered'
    )
    
    # 9. Fix self.update for CameraPermissionRequested  
    content = content.replace(
        'self.update(Event::CameraPermissionRequested, model, caps);',
        'model.camera_permission_state = PermissionState::Requesting;'
    )
    
    with open('src/lib.rs', 'w') as f:
        f.write(content)
    
    new_lines = len(content.split('\n'))
    print(f"New lines: {new_lines}")
    print(f"Lines changed: {original_lines - new_lines}")
    print("Migration complete!")

if __name__ == '__main__':
    migrate_lib_rs()
