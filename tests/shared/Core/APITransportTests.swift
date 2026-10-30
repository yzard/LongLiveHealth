import Foundation
#if canImport(FoundationNetworking)
import FoundationNetworking
#endif
import Testing
@testable import LongLiveHealthCore

@Test func sharedHealthUploadUsesInjectedPlatformTransport() async throws {
    let server = try #require(URL(string: "https://example.invalid"))
    let body: JSONValue = .object(["batch_id": .string("batch-1"), "records": .array([.object(["name": .string("心率")])])])
    let client = APIClient(server: server, token: "synthetic-token", uploadTransport: { request, data, operation in
        #expect(request.url?.path == "/api/v1/health/sync")
        #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer synthetic-token")
        #expect(operation == "batch-1")
        #expect(try JSONDecoder().decode(JSONValue.self, from: data) == body)
        let response = try #require(HTTPURLResponse(url: server, statusCode: 200, httpVersion: nil, headerFields: nil))
        return (Data("{\"total_records\":1}".utf8), response)
    })
    let reply = try await client.post("health/sync", body: body)
    #expect(reply["total_records"].numberValue == 1)
}

@Test func platformUploadFailureRetainsServerReason() async throws {
    let server = try #require(URL(string: "https://example.invalid"))
    let client = APIClient(server: server, token: nil, uploadTransport: { _, _, _ in
        let response = try #require(HTTPURLResponse(url: server, statusCode: 409, httpVersion: nil, headerFields: nil))
        return (Data("{\"error\":{\"message\":\"Record conflict\"}}".utf8), response)
    })
    do {
        _ = try await client.post("health/sync", body: .object(["batch_id": .string("batch-2"), "records": .array([.object([:])])]))
        Issue.record("Expected rejected upload")
    } catch APIError.status(let code, let reason) {
        #expect(code == 409)
        #expect(reason == "Record conflict")
    }
}
