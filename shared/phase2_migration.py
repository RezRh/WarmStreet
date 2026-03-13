#!/usr/bin/env python3
"""
Phase 2 Migration Script - Complete Codebase Optimization
Removes duplicate definitions, fixes Telemetry, updates to new types
"""
import re

def remove_duplicate_definitions():
    """Remove duplicate struct/enum definitions from lib.rs (lines ~532-2517)"""
    with open('src/lib.rs', 'r') as f:
        lines = f.readlines()
    
    # Keep lines 1-531 and 2518+
    # But we need to be smarter - find the actual boundaries
    
    new_lines = []
    in_duplicate_section = False
    brace_depth = 0
    found_app_impl = False
    
    for i, line in enumerate(lines, 1):
        # Start of duplicate section - after module declarations
        if i > 500 and 'pub enum CoordinateError' in line:
            in_duplicate_section = True
            continue
        
        # End of duplicate section - when we find the App impl
        if in_duplicate_section and 'impl crux_core::App for App' in line:
            in_duplicate_section = False
            found_app_impl = True
        
        if not in_duplicate_section:
            new_lines.append(line)
    
    with open('src/lib.rs', 'w') as f:
        f.writelines(new_lines)
    
    print(f"Removed duplicate definitions. New line count: {len(new_lines)}")

def fix_telemetry():
    """Replace telemetry calls with tracing or remove them"""
    with open('src/lib.rs', 'r') as f:
        content = f.read()
    
    # Replace all caps.telemetry() calls with tracing or just remove
    content = re.sub(r'caps\.telemetry\(\)\.event\([^)]+\);', '; // telemetry', content)
    content = re.sub(r'caps\.telemetry\(\)\.error\([^)]+\);', '; // telemetry', content)
    content = re.sub(r'caps\.telemetry\(\)\.warn\([^)]+\);', '; // telemetry', content)
    content = re.sub(r'caps\.telemetry\(\)\.counter\([^)]+\);', '; // telemetry', content)
    content = re.sub(r'caps\.telemetry\(\)\.gauge\([^)]+\);', '; // telemetry', content)
    
    with open('src/lib.rs', 'w') as f:
        f.write(content)
    
    print("Fixed telemetry calls")

def update_to_new_types():
    """Update lib.rs to use new model types"""
    with open('src/lib.rs', 'r') as f:
        content = f.read()
    
    # Replace ValidatedCoordinate with LatLon where appropriate
    # Replace haversine_distance calls with LatLon::haversine_distance method
    content = content.replace(
        'haversine_distance(self, other)',
        'self.haversine_distance(&other)'
    )
    
    # Replace format_time_ago with chrono-humanize
    content = content.replace(
        'format_time_ago(',
        '// format_time_ago( - use chrono-humanize instead\n        //'
    )
    
    with open('src/lib.rs', 'w') as f:
        f.write(content)
    
    print("Updated to new types")

def update_outbox_for_governor():
    """Update outbox.rs to use governor for rate limiting"""
    with open('src/outbox.rs', 'r') as f:
        content = f.read()
    
    # Add governor import
    if 'use governor' not in content:
        content = 'use governor::{Quota, RateLimiter};\nuse std::num::NonZeroU32;\n' + content
    
    # Replace custom RateLimiter with governor
    # This is a simplified replacement - actual implementation may vary
    content = re.sub(
        r'struct RateLimiter \{[^}]+\}',
        '// RateLimiter replaced with governor crate',
        content
    )
    
    with open('src/outbox.rs', 'w') as f:
        f.write(content)
    
    print("Updated outbox.rs for governor")

def main():
    print("Starting Phase 2 migration...")
    remove_duplicate_definitions()
    fix_telemetry()
    update_to_new_types()
    update_outbox_for_governor()
    print("Phase 2 migration complete!")

if __name__ == '__main__':
    main()
