import Foundation
import Shared

class KeyValueHandler {

    func handle(_ operation: KeyValueOperation) async -> Event {
        switch operation {
        case .get(let key):
            let value = read(key: key)
            return Event.keyValue(.get(value: value))
            
        case .set(let key, let value):
            save(key: key, data: value)
            return Event.keyValue(.set)
            
        case .delete(let key):
            delete(key: key)
            return Event.keyValue(.delete)
            
        case .exists(let key):
            let exists = read(key: key) != nil
            return Event.keyValue(.exists(exists: exists))
            
        case .listKeys(let prefix, let cursor):
            // Keychain listing is complex and not fully supported for prefix search without iterating all
            // For now, implementing basic listing if feasible or stubbing
            return Event.keyValue(.listKeys(keys: [], nextCursor: 0))
        }
    }

    private func save(key: String, data: Data) {
        let query = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecValueData as String: data
        ] as [String: Any]

        SecItemDelete(query as CFDictionary)
        SecItemAdd(query as CFDictionary, nil)
    }

    private func read(key: String) -> Data? {
        let query = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String: kCFBooleanTrue!,
            kSecMatchLimit as String: kSecMatchLimitOne
        ] as [String: Any]

        var dataTypeRef: AnyObject?
        let status: OSStatus = SecItemCopyMatching(query as CFDictionary, &dataTypeRef)

        if status == noErr {
            return dataTypeRef as? Data
        }
        return nil
    }
    
    private func delete(key: String) {
        let query = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key
        ] as [String: Any]

        SecItemDelete(query as CFDictionary)
    }
}
