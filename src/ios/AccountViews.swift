import SwiftUI

struct PasswordChangeView: View {
    var model: AppModel
    let required: Bool
    @Environment(\.dismiss) private var dismiss
    @State private var current = ""
    @State private var replacement = ""
    @State private var confirmation = ""
    var body: some View {
        Form {
            Text(required ? "Change the initial password before using your archive." : "Changing your password signs out every device. Sign in again with your new password.")
            SecureField("Current password", text: $current).textContentType(.password)
            SecureField("New password (at least 12 characters)", text: $replacement).textContentType(.newPassword)
            SecureField("Confirm new password", text: $confirmation).textContentType(.newPassword)
            Button("Change password and sign out") {
                Task {
                    await model.changePassword(current: current, replacement: replacement)
                    if model.session == nil { current = ""; replacement = ""; confirmation = ""; dismiss() }
                }
            }.disabled(model.isBusy || model.isHealthSyncRunning || current.isEmpty || replacement.utf8.count < 12 || replacement.utf8.count > 1024 || replacement != confirmation || current == replacement)
            if required { Button("Sign out") { Task { await model.logout() } }.disabled(model.isBusy) }
        }
        .navigationTitle("Change password")
        .interactiveDismissDisabled(required || !current.isEmpty || !replacement.isEmpty || !confirmation.isEmpty)
        .toolbar { if !required { Button("Cancel", role: .cancel) { dismiss() }.disabled(model.isBusy) } }
    }
}

struct UserManagementView: View {
    var model: AppModel
    @Environment(\.dismiss) private var dismiss
    @State private var users: [JSONValue] = []
    @State private var loaded = false
    @State private var creating = false
    @State private var deleting: JSONValue?
    @State private var failure: String?
    var body: some View {
        List {
            if let failure { Text(failure).foregroundStyle(.red) }
            ForEach(users, id: \.identifier) { user in
                HStack {
                    Text(user["username"].stringValue)
                    Spacer()
                    if user["is_admin"].boolValue { Text("Administrator").foregroundStyle(.secondary) }
                    else { Button("Delete", role: .destructive) { deleting = user } }
                }
            }
        }.navigationTitle("Users")
            .task { if !loaded { await load() } }
            .refreshable { await load() }
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Done") { dismiss() } }
                ToolbarItem(placement: .primaryAction) { Button("Add user") { creating = true } }
            }
            .sheet(isPresented: $creating) { NavigationStack { CreateUserView(model: model) { user in users.append(user) } } }
            .confirmationDialog("Delete this user and all their server data?", isPresented: Binding(get: { deleting != nil }, set: { if !$0 { deleting = nil } })) {
                if let user = deleting { Button("Delete \(user["username"].stringValue)", role: .destructive) { Task { await remove(user) } } }
            }
    }
    private func load() async {
        do {
            var result: [JSONValue] = []; var cursor: JSONValue = .null
            while true {
                let page = try await model.request("users/list", body: .object(["limit": .number(100), "after_id": cursor]))["users"].arrayValue
                result.append(contentsOf: page)
                if page.count < 100 { break }
                cursor = page.last?["user_id"] ?? .null
            }
            users = result; loaded = true; failure = nil
        } catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
    private func remove(_ user: JSONValue) async {
        do {
            let result = try await model.request("users/delete", body: .object(["user_id": user["user_id"], "confirmation": user["username"]]))
            users.removeAll { $0["user_id"] == result["user_id"] }
        } catch { failure = error.localizedDescription }
    }
}

private struct CreateUserView: View {
    var model: AppModel
    let onSave: (JSONValue) -> Void
    @Environment(\.dismiss) private var dismiss
    @State private var username = ""
    @State private var password = ""
    @State private var saving = false
    @State private var failure: String?
    var body: some View {
        Form {
            TextField("Username", text: $username).textInputAutocapitalization(.never).autocorrectionDisabled()
            SecureField("Password (at least 12 characters)", text: $password).textContentType(.newPassword)
            if let failure { Text(failure).foregroundStyle(.red) }
            Button("Create user") { Task { await save() } }.disabled(saving || username.isEmpty || password.utf8.count < 12)
        }.navigationTitle("Add user").disabled(saving)
            .interactiveDismissDisabled(saving || !username.isEmpty || !password.isEmpty)
            .toolbar { Button("Cancel", role: .cancel) { dismiss() }.disabled(saving) }
    }
    private func save() async {
        saving = true; defer { saving = false }
        do {
            let user = try await model.request("users/create", body: .object(["username": .string(username), "password": .string(password)]))
            onSave(user); password = ""; dismiss()
        } catch { failure = error.localizedDescription }
    }
}
