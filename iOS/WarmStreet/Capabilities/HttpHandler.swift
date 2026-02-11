import Foundation
import Shared

class HttpHandler {
    
    func handle(_ request: HttpRequest) async -> Event {
        guard let url = URL(string: request.url) else {
            // Handle error logic, possibly return failure event if possible
            return Event.http(.failure(error: "Invalid URL"))
        }
        
        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = request.method
        urlRequest.httpBody = request.body
        
        for header in request.headers {
            urlRequest.setValue(header.value, forHTTPHeaderField: header.name)
        }
        
        do {
            let (data, response) = try await URLSession.shared.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                return Event.http(.failure(error: "Not HTTP response"))
            }
            
            let status = httpResponse.statusCode
            // Map headers back if needed
            
            return Event.http(.success(status: UInt16(status), body: data))
        } catch {
            return Event.http(.failure(error: error.localizedDescription))
        }
    }
}
