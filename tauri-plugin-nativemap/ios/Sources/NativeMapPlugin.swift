// NativeMapPlugin.swift
// tauri-plugin-nativemap / iOS
//
// Bridges Tauri commands to Apple Maps (MKMapView).
// This file is compiled by the Tauri iOS build system.
//
// Architecture:
//   Tauri Backend (Rust) → MapOperation → this plugin
//   User taps pin → this plugin → "map-pin-tapped" event → Backend MapPinTapped

import UIKit
import MapKit
import WebKit

/// Tauri plugin: NativeMapPlugin
/// Registered automatically by tauri-plugin-nativemap's init() in Rust.
@objc public class NativeMapPlugin: NSObject {

    // MARK: - Properties
    private var mapView: MKMapView?
    private var webView: WKWebView?
    private var pinIdsByAnnotation = [MKAnnotation: String]()
    private var pinTappedCallback: ((String) -> Void)?

    // MARK: - Initialisation

    @objc public func setup(webView: WKWebView, pinTapped: @escaping (String) -> Void) {
        self.webView = webView
        self.pinTappedCallback = pinTapped
    }

    // MARK: - Commands (called from Rust via Tauri mobile bridge)

    /// Show or re-configure the native MKMapView overlay.
    @objc public func showMap(_ configJson: String) {
        guard let data = configJson.data(using: .utf8),
              let config = try? JSONDecoder().decode(MapConfig.self, from: data) else {
            NSLog("NativeMapPlugin.showMap: failed to decode config")
            return
        }

        DispatchQueue.main.async { [weak self] in
            guard let self else { return }

            // If map already exists just recenter it.
            if let existing = self.mapView {
                existing.isHidden = false
                self.center(existing, lat: config.centerLat, lon: config.centerLon, zoom: config.zoom)
                if config.showUserLocation { existing.showsUserLocation = true }
                return
            }

            guard let parent = self.webView?.superview else { return }

            let map = MKMapView(frame: parent.bounds)
            map.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            map.delegate = self
            map.showsUserLocation = config.showUserLocation
            map.mapType = config.satelliteMode ? .satellite : .standard

            // Insert behind the WebView so the Liquid Glass nav is still on top.
            parent.insertSubview(map, belowSubview: self.webView!)
            self.mapView = map

            self.center(map, lat: config.centerLat, lon: config.centerLon, zoom: config.zoom)

            // Draw the search-radius circle.
            if config.searchRadiusM > 0 {
                let circle = MKCircle(
                    center: CLLocationCoordinate2D(latitude: config.centerLat, longitude: config.centerLon),
                    radius: CLLocationDistance(config.searchRadiusM)
                )
                map.addOverlay(circle)
            }

            NSLog("NativeMapPlugin: MKMapView ready")
        }
    }

    /// Replace all annotation pins.
    @objc public func updatePins(_ pinsJson: String) {
        guard let data = pinsJson.data(using: .utf8),
              let pins = try? JSONDecoder().decode([MapPin].self, from: data) else {
            NSLog("NativeMapPlugin.updatePins: failed to decode pins")
            return
        }

        DispatchQueue.main.async { [weak self] in
            guard let self, let map = self.mapView else { return }

            // Remove existing annotations (keep user location).
            let toRemove = map.annotations.filter { !($0 is MKUserLocation) }
            map.removeAnnotations(toRemove)
            self.pinIdsByAnnotation.removeAll()

            for pin in pins {
                let ann = CaseAnnotation(
                    caseId: pin.id,
                    title: pin.title,
                    subtitle: pin.subtitle,
                    severity: pin.severity,
                    coordinate: CLLocationCoordinate2D(latitude: pin.lat, longitude: pin.lon)
                )
                self.pinIdsByAnnotation[ann] = pin.id
                map.addAnnotation(ann)
            }
            NSLog("NativeMapPlugin: updated \(pins.count) pins")
        }
    }

    /// Hide the map view (keep it in memory for quick re-show).
    @objc public func hideMap() {
        DispatchQueue.main.async { [weak self] in
            self?.mapView?.isHidden = true
            NSLog("NativeMapPlugin: map hidden")
        }
    }

    /// Pan camera to a new centre.
    @objc public func panTo(lat: Double, lon: Double) {
        DispatchQueue.main.async { [weak self] in
            guard let map = self?.mapView else { return }
            let region = MKCoordinateRegion(
                center: CLLocationCoordinate2D(latitude: lat, longitude: lon),
                latitudinalMeters: 2000,
                longitudinalMeters: 2000
            )
            map.setRegion(region, animated: true)
        }
    }

    // MARK: - Private helpers

    private func center(_ map: MKMapView, lat: Double, lon: Double, zoom: Double) {
        let span = spanFromZoom(zoom)
        let region = MKCoordinateRegion(
            center: CLLocationCoordinate2D(latitude: lat, longitude: lon),
            span: span
        )
        map.setRegion(region, animated: false)
    }

    private func spanFromZoom(_ zoom: Double) -> MKCoordinateSpan {
        // Convert a "zoom level" (Google Maps convention) to a delta.
        let delta = 360.0 / pow(2.0, zoom)
        return MKCoordinateSpan(latitudeDelta: delta, longitudeDelta: delta)
    }
}

// MARK: - MKMapViewDelegate

extension NativeMapPlugin: MKMapViewDelegate {

    public func mapView(_ mapView: MKMapView, viewFor annotation: MKAnnotation) -> MKAnnotationView? {
        guard let caseAnn = annotation as? CaseAnnotation else { return nil }

        let id = "CasePin"
        let view = mapView.dequeueReusableAnnotationView(withIdentifier: id) as? MKMarkerAnnotationView
                   ?? MKMarkerAnnotationView(annotation: annotation, reuseIdentifier: id)

        view.annotation = annotation
        view.canShowCallout = true
        view.glyphText = nil

        // Pin colour by severity.
        switch caseAnn.severity {
        case "Critical": view.markerTintColor = UIColor.systemRed
        case "High":     view.markerTintColor = UIColor.systemPurple
        case "Moderate": view.markerTintColor = UIColor.systemPurple.withAlphaComponent(0.7)
        case "Low":      view.markerTintColor = UIColor.systemGray
        default:         view.markerTintColor = UIColor.systemGray
        }

        return view
    }

    public func mapView(_ mapView: MKMapView, rendererFor overlay: MKOverlay) -> MKOverlayRenderer {
        if let circle = overlay as? MKCircle {
            let renderer = MKCircleRenderer(circle: circle)
            renderer.fillColor = UIColor.systemPurple.withAlphaComponent(0.08)
            renderer.strokeColor = UIColor.systemPurple.withAlphaComponent(0.3)
            renderer.lineWidth = 1.5
            return renderer
        }
        return MKOverlayRenderer(overlay: overlay)
    }

    public func mapView(_ mapView: MKMapView, didSelect view: MKAnnotationView) {
        guard let ann = view.annotation as? CaseAnnotation else { return }
        NSLog("NativeMapPlugin: pin tapped — caseId=\(ann.caseId)")
        pinTappedCallback?(ann.caseId)
    }
}

// MARK: - Model types (decoded from Tauri events)

private struct MapConfig: Decodable {
    let centerLat: Double
    let centerLon: Double
    let zoom: Double
    let showUserLocation: Bool
    let searchRadiusM: UInt32
    let satelliteMode: Bool

    enum CodingKeys: String, CodingKey {
        case centerLat = "center_lat"
        case centerLon = "center_lon"
        case zoom
        case showUserLocation = "show_user_location"
        case searchRadiusM = "search_radius_m"
        case satelliteMode = "satellite_mode"
    }
}

private struct MapPin: Decodable {
    let id: String
    let lat: Double
    let lon: Double
    let severity: String
    let title: String
    let subtitle: String?
}

// MARK: - CaseAnnotation

private class CaseAnnotation: NSObject, MKAnnotation {
    let caseId: String
    let severity: String
    var coordinate: CLLocationCoordinate2D
    var title: String?
    var subtitle: String?

    init(caseId: String, title: String, subtitle: String?, severity: String, coordinate: CLLocationCoordinate2D) {
        self.caseId = caseId
        self.severity = severity
        self.coordinate = coordinate
        self.title = title
        self.subtitle = subtitle
    }
}
