import Foundation
import UserNotifications
import UIKit

class PushHandler: NSObject, UNUserNotificationCenterDelegate {
    static let shared = PushHandler()
    var core: Core?

    func requestAuthorization() {
        UNUserNotificationCenter.current().delegate = self
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, _ in
            if granted {
                DispatchQueue.main.async {
                    UIApplication.shared.registerForRemoteNotifications()
                }
            }
        }
    }

    func userNotificationCenter(_ center: UNUserNotificationCenter, willPresent notification: UNNotification, withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void) {
        let userInfo = notification.request.content.userInfo
        handlePush(userInfo: userInfo)
        completionHandler([.banner, .list, .sound])
    }

    func userNotificationCenter(_ center: UNUserNotificationCenter, didReceive response: UNNotificationResponse, withCompletionHandler completionHandler: @escaping () -> Void) {
        let userInfo = response.notification.request.content.userInfo
        handlePush(userInfo: userInfo)
        completionHandler()
    }

    private func handlePush(userInfo: [AnyHashable: Any]) {
        guard let type = userInfo["type"] as? String else { return }
        
        let payload: PushPayload
        
        switch type {
        case "new_rescue":
            guard let caseId = userInfo["case_id"] as? String,
                  let latStr = userInfo["lat"] as? String,
                  let lngStr = userInfo["lng"] as? String,
                  let lat = Double(latStr),
                  let lng = Double(lngStr) else { return }
            payload = .newRescue(caseId: caseId, lat: lat, lng: lng)
            
        case "mute":
            guard let caseId = userInfo["case_id"] as? String,
                  let claimedBy = userInfo["claimed_by"] as? String else { return }
            payload = .mute(caseId: caseId, claimedBy: claimedBy)
            
        case "case_update":
            guard let caseId = userInfo["case_id"] as? String,
                  let newStatus = userInfo["new_status"] as? String else { return }
            payload = .caseUpdate(caseId: caseId, newStatus: newStatus)
            
        default:
            return
        }
        
        core?.update(.pushReceived(payload))
    }
}
