// NativeMapPlugin.kt
// tauri-plugin-nativemap / Android
//
// Bridges Tauri events to the Google Maps SDK for Android.
// Google Maps SDK for Android is FREE for basic map display (no billing required).
//
// Architecture:
//   Tauri Backend (Rust) → MapOperation → this plugin
//   User taps pin → this plugin → Tauri "map-pin-tapped" event → Backend MapPinTapped
//
// Prerequisites:
//   1. Add `com.google.android.gms:play-services-maps` to android/app/build.gradle
//   2. Add your API key to android/app/src/main/res/values/strings.xml

package app.warmstreet.nativemap

import android.annotation.SuppressLint
import android.os.Bundle
import android.view.ViewGroup
import android.widget.FrameLayout
import androidx.fragment.app.FragmentActivity
import com.google.android.gms.maps.CameraUpdateFactory
import com.google.android.gms.maps.GoogleMap
import com.google.android.gms.maps.OnMapReadyCallback
import com.google.android.gms.maps.SupportMapFragment
import com.google.android.gms.maps.model.*
import org.json.JSONArray
import org.json.JSONObject

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

private fun severityHue(severity: String): Float = when (severity) {
    "Critical" -> BitmapDescriptorFactory.HUE_RED
    "High"     -> 270f  // violet-ish
    "Moderate" -> 280f  // lighter violet
    else       -> BitmapDescriptorFactory.HUE_AZURE
}

// ---------------------------------------------------------------------------
// NativeMapPlugin
// ---------------------------------------------------------------------------

class NativeMapPlugin(
    private val activity: FragmentActivity,
    private val webViewContainer: ViewGroup,
    private val onPinTapped: (String) -> Unit,
) : OnMapReadyCallback {

    private var googleMap: GoogleMap? = null
    private var mapFragment: SupportMapFragment? = null
    private var mapContainer: FrameLayout? = null
    private val pinIds = mutableMapOf<Marker, String>()

    // -----------------------------------------------------------------------
    // Public API (called from Rust via Tauri)
    // -----------------------------------------------------------------------

    @SuppressLint("MissingPermission")
    fun showMap(configJson: String) {
        val config = JSONObject(configJson)
        val centerLat    = config.getDouble("center_lat")
        val centerLon    = config.getDouble("center_lon")
        val zoom         = config.getDouble("zoom").toFloat()
        val showUserLoc  = config.optBoolean("show_user_location", true)
        val radiusM      = config.optInt("search_radius_m", 5000).toDouble()
        val satellite    = config.optBoolean("satellite_mode", false)

        activity.runOnUiThread {
            // If the map is already up, just update it.
            googleMap?.let { map ->
                map.isHidden = false
                mapContainer?.visibility = android.view.View.VISIBLE
                moveCamera(map, centerLat, centerLon, zoom)
                if (showUserLoc) tryEnableUserLocation(map)
                return@runOnUiThread
            }

            // Create a container for the map fragment below the WebView.
            val container = FrameLayout(activity).also {
                it.layoutParams = ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT
                )
                it.id = android.view.View.generateViewId()
            }
            // Insert below the WebView so the Liquid Glass nav stays on top.
            webViewContainer.addView(container, 0)
            mapContainer = container

            val fragment = SupportMapFragment.newInstance()
            activity.supportFragmentManager
                .beginTransaction()
                .add(container.id, fragment)
                .commit()
            fragment.getMapAsync(this)
            mapFragment = fragment

            // Store config for use once map is ready.
            pendingCenter = Triple(centerLat, centerLon, zoom)
            pendingShowUserLoc = showUserLoc
            pendingRadius = radiusM
            pendingSatellite = satellite
        }
    }

    fun updatePins(pinsJson: String) {
        val pins = JSONArray(pinsJson)
        activity.runOnUiThread {
            val map = googleMap ?: return@runOnUiThread
            map.clear()
            pinIds.clear()

            for (i in 0 until pins.length()) {
                val p = pins.getJSONObject(i)
                val id       = p.getString("id")
                val lat      = p.getDouble("lat")
                val lon      = p.getDouble("lon")
                val severity = p.optString("severity", "Low")
                val title    = p.optString("title", "Animal in distress")
                val subtitle = p.optString("subtitle", "")

                val marker = map.addMarker(
                    MarkerOptions()
                        .position(LatLng(lat, lon))
                        .title(title)
                        .snippet(subtitle)
                        .icon(BitmapDescriptorFactory.defaultMarker(severityHue(severity)))
                ) ?: continue
                pinIds[marker] = id
            }
        }
    }

    fun hideMap() {
        activity.runOnUiThread {
            mapContainer?.visibility = android.view.View.GONE
        }
    }

    fun panTo(lat: Double, lon: Double) {
        activity.runOnUiThread {
            googleMap?.animateCamera(CameraUpdateFactory.newLatLng(LatLng(lat, lon)))
        }
    }

    fun setZoom(level: Float) {
        activity.runOnUiThread {
            googleMap?.animateCamera(CameraUpdateFactory.zoomTo(level))
        }
    }

    // -----------------------------------------------------------------------
    // OnMapReadyCallback
    // -----------------------------------------------------------------------

    @SuppressLint("MissingPermission")
    override fun onMapReady(map: GoogleMap) {
        googleMap = map

        pendingSatellite?.let { sat ->
            map.mapType = if (sat) GoogleMap.MAP_TYPE_SATELLITE else GoogleMap.MAP_TYPE_NORMAL
        }
        pendingCenter?.let { (lat, lon, zoom) -> moveCamera(map, lat, lon, zoom) }
        pendingShowUserLoc?.let { tryEnableUserLocation(map) }
        pendingRadius?.let { radius ->
            val centre = pendingCenter ?: return@let
            map.addCircle(
                CircleOptions()
                    .center(LatLng(centre.first, centre.second))
                    .radius(radius)
                    .fillColor(0x14A855F7.toInt())  // violet, 8% opacity
                    .strokeColor(0x4DA855F7.toInt())
                    .strokeWidth(2f)
            )
        }

        // Forward marker taps to the backend.
        map.setOnMarkerClickListener { marker ->
            pinIds[marker]?.let { caseId -> onPinTapped(caseId) }
            false
        }

        android.util.Log.i("NativeMapPlugin", "GoogleMap ready")
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    private var pendingCenter: Triple<Double, Double, Float>? = null
    private var pendingShowUserLoc: Boolean? = null
    private var pendingRadius: Double? = null
    private var pendingSatellite: Boolean? = null

    private fun moveCamera(map: GoogleMap, lat: Double, lon: Double, zoom: Float) {
        map.moveCamera(CameraUpdateFactory.newLatLngZoom(LatLng(lat, lon), zoom))
    }

    @SuppressLint("MissingPermission")
    private fun tryEnableUserLocation(map: GoogleMap) {
        try {
            map.isMyLocationEnabled = true
        } catch (e: SecurityException) {
            android.util.Log.w("NativeMapPlugin", "Location permission not granted: ${e.message}")
        }
    }
}
