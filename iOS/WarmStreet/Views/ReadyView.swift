import SwiftUI
import Shared

struct ReadyView: View {
    @EnvironmentObject var core: Core
    
    var body: some View {
        VStack {
            Image(systemName: "checkmark.circle.fill")
                .resizable()
                .frame(width: 100, height: 100)
                .foregroundColor(.green)
                .padding()
            
            Text("You're All Set!")
                .font(.largeTitle)
                .fontWeight(.bold)
            
            Text("Welcome to WarmStreet.")
                .font(.title2)
                .foregroundColor(.gray)
            
            if case .ready(let cases, let online) = core.view.state {
                 Text(online ? "Online" : "Offline")
                    .foregroundColor(online ? .green : .red)
                    .padding()
                 
                 List(cases, id: \.localId) { caseItem in
                     VStack(alignment: .leading) {
                         Text(caseItem.description ?? "No description")
                         Text(caseItem.status).font(.caption).foregroundColor(.gray)
                     }
                 }
            }
        }
    }
}
