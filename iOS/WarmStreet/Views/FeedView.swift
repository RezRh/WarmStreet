import SwiftUI
import Shared

struct FeedView: View {
    @EnvironmentObject var core: Core
    
    // Core state drives the view mode (Map/List)
    // We can use a local tab selection for the main bottom bar
    // But the Map/List toggle might be a segmented control at top or the bottom bar itself?
    // Requirement: "Bottom tab bar: Map | List | Report (+ button) | Profile"
    
    @State private var showingReport = false
    
    var body: some View {
        TabView(selection: Binding(
            get: { 
                return core.view.feedView == "Map" ? 0 : 1
            },
            set: { val in
                if val == 0 { core.update(.switchToMap) }
                else if val == 1 { core.update(.switchToList) }
                else if val == 2 {
                    // Trigger Photo Capture
                    core.update(.capturePhotoRequested)
                }
            }
        )) {
            MapFeedView()
                .tabItem {
                    Label("Map", systemImage: "map")
                }
                .tag(0)
            
            ListFeedView()
                .tabItem {
                    Label("List", systemImage: "list.bullet")
                }
                .tag(1)
            
            Color.clear // Report placeholder
                .tabItem {
                    Label("Report", systemImage: "plus.circle.fill")
                }
                .tag(2)
            
            Text("Profile")
                .tabItem {
                    Label("Profile", systemImage: "person.crop.circle")
                }
                .tag(3)
        }
        .sheet(item: Binding(
            get: { core.view.selectedDetail.map { DetailWrapper(detail: $0) } },
            set: { if $0 == nil { core.update(.caseDismissed) } }
        )) { wrapper in
            CaseDetailSheet(detail: wrapper.detail)
        }
        .sheet(isPresented: Binding(
            get: { core.view.stagedPhoto != nil },
            set: { if !$0 { core.update(.photoCancelled) } }
        )) {
            ReportView()
                .environmentObject(core)
        }
    }
}

struct DetailWrapper: Identifiable {
    let detail: CaseDetail
    var id: String { detail.id }
}

extension ViewModel {
    var feedView: String {
        if case let .ready(feedView, _, _, _, _, _, _, _) = state {
            return feedView
        }
        return "Map"
    }
    
    var selectedDetail: CaseDetail? {
        if case let .ready(_, _, _, detail, _, _, _, _) = state {
            return detail
        }
        return nil
    }
}
