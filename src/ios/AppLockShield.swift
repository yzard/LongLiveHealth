import UIKit

/// A separate opaque window protects tabs, presented sheets and app-switcher snapshots.
@MainActor final class AppLockShield {
    static let shared = AppLockShield()
    private var window: UIWindow?

    func update(model: AppModel) {
        guard model.isLocked || model.privacyCovered else {
            window?.isHidden = true; window = nil
            return
        }
        guard let scene = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene }).first(where: { $0.activationState == .foregroundActive || $0.activationState == .foregroundInactive }) else { return }
        let shield = window ?? UIWindow(windowScene: scene)
        shield.windowLevel = .alert + 1
        let controller = UIViewController()
        controller.view.backgroundColor = .systemBackground
        controller.view.accessibilityViewIsModal = true
        let title = UILabel(); title.text = "Your health archive is locked"; title.font = .preferredFont(forTextStyle: .headline)
        title.adjustsFontForContentSizeCategory = true; title.numberOfLines = 0; title.textAlignment = .center
        let unlock = UIButton(type: .system)
        unlock.setTitle("Unlock with Face ID / Touch ID", for: .normal)
        unlock.addAction(UIAction { [weak model] _ in Task { await model?.unlock() } }, for: .touchUpInside)
        let signOut = UIButton(type: .system); signOut.setTitle("Sign out", for: .normal)
        signOut.addAction(UIAction { [weak model] _ in Task { await model?.logout() } }, for: .touchUpInside)
        unlock.isHidden = model.privacyCovered; signOut.isHidden = model.privacyCovered
        let stack = UIStackView(arrangedSubviews: [title, unlock, signOut]); stack.axis = .vertical; stack.spacing = 24
        stack.translatesAutoresizingMaskIntoConstraints = false
        controller.view.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.centerYAnchor.constraint(equalTo: controller.view.centerYAnchor),
            stack.leadingAnchor.constraint(equalTo: controller.view.safeAreaLayoutGuide.leadingAnchor, constant: 24),
            stack.trailingAnchor.constraint(equalTo: controller.view.safeAreaLayoutGuide.trailingAnchor, constant: -24)
        ])
        shield.rootViewController = controller; shield.isHidden = false; window = shield
    }
}
