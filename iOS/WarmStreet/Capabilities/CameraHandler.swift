import Foundation
import UIKit
import Shared
import SwiftUI

class CameraHandler: NSObject, UIImagePickerControllerDelegate, UINavigationControllerDelegate {
    var onPhotoCaptured: ((Data, UInt32, UInt32) -> Void)?
    
    func presentCamera(from viewController: UIViewController, callback: @escaping (Data, UInt32, UInt32) -> Void) {
        self.onPhotoCaptured = callback
        
        if UIImagePickerController.isSourceTypeAvailable(.camera) {
            let picker = UIImagePickerController()
            picker.sourceType = .camera
            picker.delegate = self
            viewController.present(picker, animated: true)
        }
    }
    
    func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey : Any]) {
        if let image = info[.originalImage] as? UIImage {
            let width = UInt32(image.size.width)
            let height = UInt32(image.size.height)
            
            if let data = image.jpegData(compressionQuality: 0.9) {
                onPhotoCaptured?(data, width, height)
            }
        }
        picker.dismiss(animated: true)
    }
    
    func imagePickerControllerDidCancel(_ picker: UIImagePickerController) {
        picker.dismiss(animated: true)
    }
}
