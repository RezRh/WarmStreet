import SwiftUI
import Shared

struct ListFeedView: View {
    @EnvironmentObject var core: Core
    
    var body: some View {
        NavigationView {
            if let items = getListItems() {
                List(items, id: \.id) { item in
                    CaseRow(item: item)
                        .onTapGesture {
                            core.update(.caseMarkerTapped(caseId: item.id))
                        }
                        .listRowSeparator(.hidden)
                }
                .listStyle(.plain)
                .refreshable {
                    core.update(.refreshRequested)
                }
                .navigationTitle("Nearby Rescues")
            } else {
                ProgressView()
            }
        }
    }
    
    func getListItems() -> [CaseListItem]? {
        if case let .ready(_, _, items, _, _, _, _, _) = core.view.state {
            return items
        }
        return nil
    }
}

struct CaseRow: View {
    let item: CaseListItem
    
    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Circle()
                .fill(colorForStatus(item.status))
                .frame(width: 12, height: 12)
                .padding(.top, 6)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(item.descriptionPreview)
                    .font(.body)
                    .lineLimit(2)
                
                HStack {
                    Image(systemName: "location.fill")
                        .font(.caption2)
                    Text(item.distanceText)
                        .font(.caption)
                    
                    Text("•")
                        .font(.caption)
                        .foregroundColor(.gray)
                    
                    Text(item.timeAgo)
                        .font(.caption)
                        .foregroundColor(.gray)
                }
                .foregroundColor(.secondary)
            }
            
            Spacer()
            
            if item.isMine {
                Image(systemName: "person.fill")
                    .foregroundColor(.blue)
            }
        }
        .padding()
        .background(Color.white) // or system background
        .cornerRadius(12)
        .shadow(color: .black.opacity(0.05), radius: 2, x: 0, y: 1)
        .padding(.vertical, 4)
    }
    
    func colorForStatus(_ status: String) -> Color {
        switch status {
        case "pending", "pending-upload": return .red
        case "synced": return .green
        default: return .gray
        }
    }
}
