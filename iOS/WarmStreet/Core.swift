import Foundation
import Shared // The generated UniFFI module

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    private let core: Shared.App // Assuming 'App' is exposed or 'process_event' wrapper
    
    // Capability Handlers
    private let httpHandler = HttpHandler()
    private let keyValueHandler: KeyValueHandler
    private let locationHandler: LocationHandler
    private let cameraHandler = CameraHandler()
    private let timeHandler = TimeHandler()

    init() {
        self.core = Shared.App() // Or appropriate initialization from generated code
        self.view = self.core.view()
        
        self.keyValueHandler = KeyValueHandler()
        self.locationHandler = LocationHandler()
        
        // Initial setup/subscription if needed
        self.locationHandler.onLocationUpdate = { [weak self] location in
             self?.update(.locationPermissionGranted(lat: location.coordinate.latitude, lng: location.coordinate.longitude))
        }
    }

    func update(_ event: Event) {
        let effects = core.update(event: event)
        self.view = core.view()
        
        Task {
            for effect in effects {
                await processEffect(effect)
            }
        }
    }
    
    private func processEffect(_ effect: Effect) async {
        switch effect {
        case .render:
            self.view = core.view()
            
        case .http(let request):
            let responseEvent = await httpHandler.handle(request)
            // Dispatch result back to core
            // Note: Since 'process_effect' is async, we call update on MainActor
            // Be careful about recursive updates. Crux handles it via message passing usually.
            // Here 'update' is synchronous with respect to state update, but effects are async.
            self.update(responseEvent)
            
        case .kv(let operation):
            let resultEvent = await keyValueHandler.handle(operation)
            self.update(resultEvent)
            
        case .camera(let operation):
            if case .capturePhoto = operation {
                // Find top view controller to present camera
                if let rootVC = await MainActor.run(body: { UIApplication.shared.windows.first?.rootViewController }) {
                    cameraHandler.presentCamera(from: rootVC) { [weak self] data, w, h in
                        let bytes = [UInt8](data)
                        self?.update(.cameraResult(.photoCaptured(bytes: bytes, width: w, height: h)))
                    }
                }
            }
            
        case .time(_):
            break
            
        case .platform(let request):
             // Handle platform specific if any
             break
             
        case .push(let operation):
            if case .requestToken = operation {
                PushHandler.shared.requestAuthorization()
                // The token will be sent back via didRegisterForRemoteNotifications -> PushHandler -> handlePush -> core.update
            }
        }
    }
}
