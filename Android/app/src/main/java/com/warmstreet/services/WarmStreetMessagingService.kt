package com.warmstreet.services

import android.util.Log
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import com.warmstreet.WarmStreetApplication

class WarmStreetMessagingService : FirebaseMessagingService() {

    override fun onNewToken(token: String) {
        super.onNewToken(token)
        Log.d("FCM", "New token: $token")
        val core = (application as? WarmStreetApplication)?.core
        core?.pushTokenReceived(token)
    }

    override fun onMessageReceived(message: RemoteMessage) {
        super.onMessageReceived(message)
        Log.d("FCM", "Message received: ${message.data}")
        
        val core = (application as? WarmStreetApplication)?.core ?: return
        val data = message.data
        
        val type = data["type"] ?: return
        
        val payload = when (type) {
            "new_rescue" -> {
                val caseId = data["case_id"] ?: return
                val lat = data["lat"]?.toDoubleOrNull() ?: return
                val lng = data["lng"]?.toDoubleOrNull() ?: return
                """{"NewRescue":{"case_id":"$caseId","lat":$lat,"lng":$lng}}"""
            }
            "mute" -> {
                val caseId = data["case_id"] ?: return
                val claimedBy = data["claimed_by"] ?: return
                """{"Mute":{"case_id":"$caseId","claimed_by":"$claimedBy"}}"""
            }
            "case_update" -> {
                val caseId = data["case_id"] ?: return
                val newStatus = data["new_status"] ?: return
                """{"CaseUpdate":{"case_id":"$caseId","new_status":"$newStatus"}}"""
            }
            else -> return
        }
        
        core?.pushReceived(payload)
    }
}
