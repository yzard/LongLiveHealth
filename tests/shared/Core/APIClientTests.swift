import Foundation
import Testing
@testable import LongLiveHealthCore

@Test func serverAddressEnforcesTransportAndRejectsEmbeddedSecrets() throws {
    #expect(try APIClient.serverURL(" https://health.example.com/base ").path == "/base")
    #expect(try APIClient.serverURL("http://localhost:8080").host == "localhost")
    #expect(try APIClient.serverURL("http://192.168.1.20:8080").host == "192.168.1.20")
    #expect(try APIClient.serverURL("http://health.example.com").scheme == "http")
    for input in ["https://secret@example.com", "https://example.com?key=secret", "https://example.com#secret", "file:///tmp"] {
        #expect(throws: (any Error).self) { try APIClient.serverURL(input) }
    }
}
