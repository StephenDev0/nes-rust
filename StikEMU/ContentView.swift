import SwiftUI
import UIKit
import UniformTypeIdentifiers
import NesRust

// MARK: - Models

struct AlertInfo: Identifiable {
    let id = UUID()
    let title: String
    let message: String
}

enum GamePlatform: String, Codable, CaseIterable, Hashable, Identifiable {
    case nes
    case ds

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .nes:
            return "Nintendo Entertainment System"
        case .ds:
            return "Nintendo DS"
        }
    }

    var coverAspectRatio: CGFloat {
        switch self {
        case .nes:
            return 720.0 / 512.0
        case .ds:
            return 135.0 / 125.0
        }
    }

    var storageFolderName: String {
            switch self {
            case .nes:
                return "Nintendo Entertainment System"
            case .ds:
                return "Nintendo DS"
            }
    }

    var isBeta: Bool {
        switch self {
        case .nes:
            return true
        case .ds:
            return true
        }
    }
}

struct GameEntry: Identifiable, Codable, Equatable {
    let id: UUID
    let name: String
    let filename: String
    let coverFilename: String?
    let platform: GamePlatform

    static func == (lhs: GameEntry, rhs: GameEntry) -> Bool {
        lhs.id == rhs.id
    }

    enum CodingKeys: String, CodingKey {
        case id, name, filename, coverFilename, platform
    }

    init(id: UUID, name: String, filename: String, coverFilename: String?, platform: GamePlatform = .nes) {
        self.id = id
        self.name = name
        self.filename = filename
        self.coverFilename = coverFilename
        self.platform = platform
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        name = try container.decode(String.self, forKey: .name)
        filename = try container.decode(String.self, forKey: .filename)
        coverFilename = try container.decodeIfPresent(String.self, forKey: .coverFilename)
        platform = try container.decodeIfPresent(GamePlatform.self, forKey: .platform) ?? .nes
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(id, forKey: .id)
        try container.encode(name, forKey: .name)
        try container.encode(filename, forKey: .filename)
        try container.encodeIfPresent(coverFilename, forKey: .coverFilename)
        try container.encode(platform, forKey: .platform)
    }
}

// MARK: - SaveStateBridge

extension Notification.Name {
    static let saveStateSlotChanged = Notification.Name("SaveStateSlotChanged")
    static let requestEmulatorReset = Notification.Name("RequestEmulatorReset")
    static let requestEmulatorStop = Notification.Name("RequestEmulatorStop")
}

final class SaveStateBridge {
    static let shared = SaveStateBridge()

    private static let slotDefaultsKey = "stikemu.currentSaveSlot"
    private static let autoSaveKey = "stikemu.autoSaveEnabled"
    private static let pauseMenuKey = "stikemu.pauseOnMenu"
    private static let legacySlotDefaultsKey = "stiknes.currentSaveSlot"
    private static let legacyAutoSaveKey = "stiknes.autoSaveEnabled"
    private static let legacyPauseMenuKey = "stiknes.pauseOnMenu"

    private(set) var slotCount: Int = 4 {
        didSet {
            slotCount = max(1, slotCount)
            if currentSlot >= slotCount {
                setCurrentSlot(slotCount - 1, notify: false)
            }
        }
    }

    private(set) var currentSlot: Int {
        didSet {
            UserDefaults.standard.set(currentSlot, forKey: SaveStateBridge.slotDefaultsKey)
        }
    }

    private(set) var autoSaveEnabled: Bool {
        didSet {
            UserDefaults.standard.set(autoSaveEnabled, forKey: SaveStateBridge.autoSaveKey)
            setAutoSaveEnabled(autoSaveEnabled)
        }
    }

    private(set) var pauseOnMenu: Bool {
        didSet {
            UserDefaults.standard.set(pauseOnMenu, forKey: SaveStateBridge.pauseMenuKey)
            setMenuPauseEnabled(pauseOnMenu)
            if menuOverlayVisible {
                setMenuOverlayVisible(menuOverlayVisible)
            }
        }
    }

    private var menuOverlayVisible = false

    private init() {
        let defaults = UserDefaults.standard
        Self.migrateLegacyDefaultsIfNeeded(in: defaults)
        let storedSlotValue = defaults.object(forKey: SaveStateBridge.slotDefaultsKey) != nil
            ? defaults.integer(forKey: SaveStateBridge.slotDefaultsKey)
            : 0
        currentSlot = max(0, min(slotCount - 1, storedSlotValue))
        // selectSaveStateSlot(Int32(currentSlot)) // Stubbed

        let nativeAutoSave = true // Stubbed isAutoSaveEnabled()
        if defaults.object(forKey: SaveStateBridge.autoSaveKey) == nil {
            defaults.set(nativeAutoSave, forKey: SaveStateBridge.autoSaveKey)
            autoSaveEnabled = nativeAutoSave
        } else {
            autoSaveEnabled = defaults.bool(forKey: SaveStateBridge.autoSaveKey)
        }
        setAutoSaveEnabled(autoSaveEnabled)

        if defaults.object(forKey: SaveStateBridge.pauseMenuKey) == nil {
            defaults.set(false, forKey: SaveStateBridge.pauseMenuKey)
        }
        pauseOnMenu = defaults.bool(forKey: SaveStateBridge.pauseMenuKey)
        setMenuPauseEnabled(pauseOnMenu)
    }

    func configure(slotCount: Int) {
        self.slotCount = max(1, slotCount)
        let stored = UserDefaults.standard.integer(forKey: SaveStateBridge.slotDefaultsKey)
        setCurrentSlot(stored, notify: true)
        setAutoSaveEnabled(autoSaveEnabled)
        setMenuPauseEnabled(pauseOnMenu)
    }

    func setCurrentSlot(_ slot: Int, notify: Bool = true) {
        let clamped = max(0, min(slotCount - 1, slot))
        if clamped != currentSlot {
            currentSlot = clamped
        }
        // selectSaveStateSlot(Int32(clamped)) // Stubbed
        if notify {
            NotificationCenter.default.post(name: .saveStateSlotChanged, object: nil, userInfo: ["slot": clamped])
        }
    }

    func quickSave() {
        // requestSaveState() // Stubbed
    }

    func quickLoad() {
        // requestLoadState() // Stubbed
    }

    func toggleAutoSave() {
        autoSaveEnabled.toggle()
    }

    func togglePauseOnMenu() {
        pauseOnMenu.toggle()
    }

    func menuOverlayVisibilityChanged(_ visible: Bool) {
        menuOverlayVisible = visible
        setMenuOverlayVisible(visible)
    }

    private static func migrateLegacyDefaultsIfNeeded(in defaults: UserDefaults) {
        if defaults.object(forKey: slotDefaultsKey) == nil,
           defaults.object(forKey: legacySlotDefaultsKey) != nil {
            defaults.set(defaults.integer(forKey: legacySlotDefaultsKey), forKey: slotDefaultsKey)
        }

        if defaults.object(forKey: autoSaveKey) == nil,
           defaults.object(forKey: legacyAutoSaveKey) != nil {
            defaults.set(defaults.bool(forKey: legacyAutoSaveKey), forKey: autoSaveKey)
        }

        if defaults.object(forKey: pauseMenuKey) == nil,
           defaults.object(forKey: legacyPauseMenuKey) != nil {
            defaults.set(defaults.bool(forKey: legacyPauseMenuKey), forKey: pauseMenuKey)
        }
    }
}

// MARK: - GameLibrary

final class GameLibrary: ObservableObject {
    @Published private(set) var games: [GameEntry] = []

    private let storageKey = "stikemu.games"
    private let legacyStorageKey = "stiknes.games"
    private let decoder = JSONDecoder()
    private let encoder = JSONEncoder()
    private var storageRootDirectory: URL
    private let fetchQueue = DispatchQueue(label: "com.stikemu.coverfetch", qos: .utility)
    private let documentsRootFolderName = "StikEMU"
    private let legacyStorageDirectoryNames = ["StikEMU-ROMs", "StikNES-ROMs"]

    init() {
        let fileManager = FileManager.default
        let documentsBase = fileManager.urls(for: .documentDirectory, in: .userDomainMask).first ?? URL(fileURLWithPath: NSTemporaryDirectory())
        storageRootDirectory = documentsBase.appendingPathComponent(documentsRootFolderName, isDirectory: true)

        try? fileManager.createDirectory(at: storageRootDirectory, withIntermediateDirectories: true)
        var resourceValues = URLResourceValues()
        resourceValues.isExcludedFromBackup = true
        try? storageRootDirectory.setResourceValues(resourceValues)

        migrateLegacyStorageDirectories(using: fileManager)
        createPlatformDirectories(using: fileManager)
        migrateStoredGamesIfNeeded()
        load()
    }

    private func directory(for platform: GamePlatform) -> URL {
        let fileManager = FileManager.default
        let directory = storageRootDirectory.appendingPathComponent(platform.storageFolderName, isDirectory: true)
        if !fileManager.fileExists(atPath: directory.path) {
            try? fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        }
        return directory
    }

    private func fileURL(for entry: GameEntry) -> URL {
        directory(for: entry.platform).appendingPathComponent(entry.filename)
    }

    private func coverURL(for entry: GameEntry, coverName: String) -> URL {
        directory(for: entry.platform).appendingPathComponent(coverName)
    }

    func addGame(from url: URL) throws {
        let accessed = url.startAccessingSecurityScopedResource()
        defer {
            if accessed {
                url.stopAccessingSecurityScopedResource()
            }
        }

        let standardizedSource = url.standardizedFileURL
        let name = url.deletingPathExtension().lastPathComponent
        let ext = url.pathExtension.lowercased()
        
        let platform: GamePlatform
        if ext == "nds" {
            platform = .ds
        } else {
            platform = .nes
        }

        let platformDirectory = directory(for: platform)
        let standardizedStorage = platformDirectory.standardizedFileURL

        if standardizedSource.path.hasPrefix(standardizedStorage.path) {
            let filename = standardizedSource.lastPathComponent
            guard games.contains(where: { $0.filename == filename }) == false else { return }
            let id = UUID()
            let entry = GameEntry(id: id, name: name, filename: filename, coverFilename: nil, platform: platform)
            games.append(entry)
            games.sort { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
            save()
            fetchCoverIfNeeded(for: entry)
            return
        }

        let id = UUID()
        let filename = "\(id.uuidString).\(ext)"
        let destination = platformDirectory.appendingPathComponent(filename)
        var retainedCover: String? = nil

        if let existing = games.first(where: { $0.name.caseInsensitiveCompare(name) == .orderedSame }) {
            if let existingCover = existing.coverFilename {
                let existingURL = coverURL(for: existing, coverName: existingCover)
                if FileManager.default.fileExists(atPath: existingURL.path) {
                    let coverExt = (existingCover as NSString).pathExtension
                    let newCoverName = "\(id.uuidString)-cover.\(coverExt)"
                    let retainedURL = platformDirectory.appendingPathComponent(newCoverName)
                    if FileManager.default.fileExists(atPath: retainedURL.path) {
                        try? FileManager.default.removeItem(at: retainedURL)
                    }
                    try? FileManager.default.copyItem(at: existingURL, to: retainedURL)
                    retainedCover = newCoverName
                }
            }
            remove(existing)
        }

        if FileManager.default.fileExists(atPath: destination.path) {
            try FileManager.default.removeItem(at: destination)
        }
        try FileManager.default.copyItem(at: url, to: destination)

        let coverFilename = copyCoverIfAvailable(from: url, id: id, platform: platform) ?? retainedCover
        let entry = GameEntry(id: id, name: name, filename: filename, coverFilename: coverFilename, platform: platform)
        games.append(entry)
        games.sort { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        save()
        fetchCoverIfNeeded(for: entry)
    }

    func remove(_ entry: GameEntry) {
        let file = fileURL(for: entry)
        if FileManager.default.fileExists(atPath: file.path) {
            try? FileManager.default.removeItem(at: file)
        }
        for slot in 0..<SaveStateBridge.shared.slotCount {
            let stateFile = saveStateURL(for: entry, slot: slot)
            if FileManager.default.fileExists(atPath: stateFile.path) {
                try? FileManager.default.removeItem(at: stateFile)
            }
        }
        let legacyState = saveStateBaseURL(for: entry)
        if FileManager.default.fileExists(atPath: legacyState.path) {
            try? FileManager.default.removeItem(at: legacyState)
        }
        if let cover = entry.coverFilename {
            let coverURL = coverURL(for: entry, coverName: cover)
            if FileManager.default.fileExists(atPath: coverURL.path) {
                try? FileManager.default.removeItem(at: coverURL)
            }
        }
        games.removeAll { $0.id == entry.id }
        games.sort { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        save()
    }

    func resolveURL(for entry: GameEntry) throws -> URL {
        let url = fileURL(for: entry)
        guard FileManager.default.fileExists(atPath: url.path) else {
            throw GameLibraryError.missingFile
        }
        return url
    }

    func saveStateBaseURL(for entry: GameEntry) -> URL {
        let stateFilename = entry.filename + ".state"
        return directory(for: entry.platform).appendingPathComponent(stateFilename)
    }

    func saveStateURL(for entry: GameEntry, slot: Int) -> URL {
        let base = saveStateBaseURL(for: entry)
        return base.appendingPathExtension("slot\(slot)")
    }

    func coverImage(for entry: GameEntry) -> UIImage? {
        guard let cover = entry.coverFilename else { return nil }
        let url = coverURL(for: entry, coverName: cover)
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        return UIImage(contentsOfFile: url.path)
    }

    private func fetchCoverIfNeeded(for entry: GameEntry) {
        guard entry.coverFilename == nil else { return }
        fetchQueue.async { [weak self] in
            guard let self = self else { return }
            let variants = self.coverNameVariants(entry.name)
            let baseURLs: [String]
            
            if entry.platform == .ds {
                baseURLs = [
                    "https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%20DS/Named_Boxarts/",
                    "https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%20DS/Named_Titles/"
                ]
            } else {
                baseURLs = [
                    "https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%20Entertainment%20System/Named_Boxarts/",
                    "https://thumbnails.libretro.com/Nintendo%20-%20Nintendo%20Entertainment%20System/Named_Titles/"
                ]
            }

            for variant in variants {
                guard let encoded = variant.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) else { continue }
                for base in baseURLs {
                    guard let remoteURL = URL(string: base + encoded + ".png") else { continue }
                    if let data = try? Data(contentsOf: remoteURL), !data.isEmpty {
                        let coverName = "\(entry.id.uuidString)-cover.png"
                        let destination = self.coverURL(for: entry, coverName: coverName)
                        do {
                            try data.write(to: destination, options: .atomic)
                            DispatchQueue.main.async {
                                self.updateEntry(withID: entry.id, coverFilename: coverName)
                            }
                        } catch {
                            try? FileManager.default.removeItem(at: destination)
                        }
                        return
                    }
                }
            }
        }
    }

    private func updateEntry(withID id: UUID, coverFilename: String?) {
        if let index = games.firstIndex(where: { $0.id == id }) {
            let entry = games[index]
            games[index] = GameEntry(id: entry.id, name: entry.name, filename: entry.filename, coverFilename: coverFilename, platform: entry.platform)
            save()
        }
    }

    private func coverNameVariants(_ name: String) -> [String] {
        var variants = Set<String>()
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        if !trimmed.isEmpty {
            variants.insert(trimmed)
        }

        let withoutUnderscores = trimmed.replacingOccurrences(of: "_", with: " ")
        if !withoutUnderscores.isEmpty {
            variants.insert(withoutUnderscores)
        }

        if let parenRange = trimmed.range(of: "(", options: .backwards) {
            let base = trimmed[..<parenRange.lowerBound].trimmingCharacters(in: .whitespacesAndNewlines)
            if !base.isEmpty {
                variants.insert(String(base))
            }
        }

        variants.insert(trimmed + " (USA)")
        variants.insert(trimmed + " (Europe)")

        return Array(variants)
    }

    private func migrateStoredGamesIfNeeded() {
        let defaults = UserDefaults.standard
        guard defaults.object(forKey: storageKey) == nil,
              let legacyData = defaults.data(forKey: legacyStorageKey) else {
            return
        }
        defaults.set(legacyData, forKey: storageKey)
    }

    private func load() {
        let defaults = UserDefaults.standard
        var data = defaults.data(forKey: storageKey)
        if data == nil, let legacyData = defaults.data(forKey: legacyStorageKey) {
            defaults.set(legacyData, forKey: storageKey)
            data = legacyData
        }

        guard let data,
              let items = try? decoder.decode([GameEntry].self, from: data) else {
            return
        }

        let fileManager = FileManager.default
        let legacyDirs = legacyDirectoryURLs()

        let filtered = items.compactMap { entry -> GameEntry? in
            let romURL = fileURL(for: entry)
            migrateLegacyFileIfNeeded(named: entry.filename, to: romURL, fileManager: fileManager, legacyDirs: legacyDirs)
            guard fileManager.fileExists(atPath: romURL.path) else { return nil }

            var coverName: String? = entry.coverFilename
            if let cover = entry.coverFilename {
                let coverPath = coverURL(for: entry, coverName: cover)
                migrateLegacyFileIfNeeded(named: cover, to: coverPath, fileManager: fileManager, legacyDirs: legacyDirs)
                if !fileManager.fileExists(atPath: coverPath.path) {
                    coverName = nil
                }
            }

            return GameEntry(id: entry.id, name: entry.name, filename: entry.filename, coverFilename: coverName, platform: entry.platform)
        }
        games = filtered.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }
        if filtered.count != items.count {
            save()
        }

        for entry in games where entry.coverFilename == nil {
            fetchCoverIfNeeded(for: entry)
        }
    }

    private func save() {
        if let data = try? encoder.encode(games) {
            UserDefaults.standard.set(data, forKey: storageKey)
        }
    }

    enum GameLibraryError: Error {
        case missingFile
    }

    private func copyCoverIfAvailable(from url: URL, id: UUID, platform: GamePlatform) -> String? {
        let possibleExtensions = ["png", "jpg", "jpeg", "webp"]
        let baseURL = url.deletingPathExtension()
        let directory = url.deletingLastPathComponent()
        let baseName = baseURL.lastPathComponent
        let altPrefixes = [baseName, "\(baseName)-cover", "\(baseName)_cover", "\(baseName)-boxart", "cover", "boxart", "front", "art"]
        let destinationDirectory = self.directory(for: platform)

        for name in altPrefixes {
            for ext in possibleExtensions {
                let candidate = directory.appendingPathComponent(name).appendingPathExtension(ext)
                if FileManager.default.fileExists(atPath: candidate.path) {
                    let coverFilename = "\(id.uuidString)-cover.\(ext)"
                    let destination = destinationDirectory.appendingPathComponent(coverFilename)
                    if FileManager.default.fileExists(atPath: destination.path) {
                        try? FileManager.default.removeItem(at: destination)
                    }
                    try? FileManager.default.copyItem(at: candidate, to: destination)
                    return coverFilename
                }
            }
        }
        return nil
    }

    private func createPlatformDirectories(using fileManager: FileManager) {
        for platform in GamePlatform.allCases {
            let directory = storageRootDirectory.appendingPathComponent(platform.storageFolderName, isDirectory: true)
            if !fileManager.fileExists(atPath: directory.path) {
                try? fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
            }
        }
    }

    private func migrateLegacyStorageDirectories(using fileManager: FileManager) {
        guard let base = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else { return }
        let legacyDirectories = legacyStorageDirectoryNames.map { base.appendingPathComponent($0, isDirectory: true) }
        let nesDirectory = directory(for: .nes)

        for legacyDir in legacyDirectories where fileManager.fileExists(atPath: legacyDir.path) {
            if let contents = try? fileManager.contentsOfDirectory(at: legacyDir, includingPropertiesForKeys: nil, options: [.skipsHiddenFiles]) {
                for item in contents {
                    let destination = nesDirectory.appendingPathComponent(item.lastPathComponent)
                    if fileManager.fileExists(atPath: destination.path) {
                        continue
                    }
                    do {
                        try fileManager.moveItem(at: item, to: destination)
                    } catch {
                        do {
                            try fileManager.copyItem(at: item, to: destination)
                            try? fileManager.removeItem(at: item)
                        } catch {
                            // Ignore failures; legacy file will be retried during load migration.
                        }
                    }
                }
            }
            try? fileManager.removeItem(at: legacyDir)
        }
    }

    private func legacyDirectoryURLs() -> [URL] {
        guard let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else { return [] }
        return legacyStorageDirectoryNames.map { base.appendingPathComponent($0, isDirectory: true) }
    }

    private func migrateLegacyFileIfNeeded(named filename: String, to destination: URL, fileManager: FileManager, legacyDirs: [URL]) {
        guard !fileManager.fileExists(atPath: destination.path) else { return }
        try? fileManager.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)

        for legacyDir in legacyDirs {
            let candidate = legacyDir.appendingPathComponent(filename)
            if fileManager.fileExists(atPath: candidate.path) {
                do {
                    try fileManager.moveItem(at: candidate, to: destination)
                } catch {
                    do {
                        try fileManager.copyItem(at: candidate, to: destination)
                        try? fileManager.removeItem(at: candidate)
                    } catch {
                        // Ignore; file may be regenerated later.
                    }
                }
                break
            }
        }
    }

}

// MARK: - Global Stubs/State

var currentEmuState: UnsafeMutableRawPointer? = nil

func requestEmulatorReset() {
    NotificationCenter.default.post(name: .requestEmulatorReset, object: nil)
}
func requestEmulatorStop() {
    NotificationCenter.default.post(name: .requestEmulatorStop, object: nil)
}
func selectSaveStateSlot(_ slot: Int32) {
    print("Slot \(slot) selected (stub)")
}
func isAutoSaveEnabled() -> Bool {
    return true
}
func setAutoSaveEnabled(_ enabled: Bool) {
    print("AutoSave \(enabled) (stub)")
}
func setMenuPauseEnabled(_ enabled: Bool) {
    print("MenuPause \(enabled) (stub)")
}
func setMenuOverlayVisible(_ visible: Bool) {
    if let state = currentEmuState {
        setEmuPaused(state, visible ? 1 : 0)
    }
}
func requestSaveState() {
     print("Save requested (stub)")
}
func requestLoadState() {
     print("Load requested (stub)")
}

// MARK: - Virtual Controller Overlay

func setVirtualButtonState(_ buttonId: Int32, _ pressed: Bool) {
    set_virtual_button_state(buttonId, pressed ? 1 : 0)
}

func releaseVirtualButtons() {
    for i in 0..<9 {
        setVirtualButtonState(Int32(i), false)
    }
}

enum VirtualButton: Int32 {
    case a = 0
    case b = 1
    case select = 2
    case start = 3
    case up = 4
    case down = 5
    case left = 6
    case right = 7
    case exit = 8
    case x = 10
    case y = 11
    case l = 12
    case r = 13
}

final class VirtualGameButton: UIButton {
    private let button: VirtualButton
    private let size: CGFloat

    init(button: VirtualButton, title: String, size: CGFloat) {
        self.button = button
        self.size = size
        super.init(frame: .zero)
        configure(title: title)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    private func configure(title: String) {
        var config = UIButton.Configuration.plain()
        config.baseForegroundColor = .white
        
        config.background.backgroundColor = UIColor.black.withAlphaComponent(0.5)
        config.background.cornerRadius = size / 2
        config.background.strokeColor = UIColor.white.withAlphaComponent(0.3)
        config.background.strokeWidth = 1.5
        
        var container = AttributeContainer()
        container.font = UIFont.systemFont(ofSize: size * 0.35, weight: .bold)
        config.attributedTitle = AttributedString(title, attributes: container)
        
        configuration = config
        
        layer.shadowColor = UIColor.black.cgColor
        layer.shadowOpacity = 0.3
        layer.shadowOffset = CGSize(width: 0, height: 4)
        layer.shadowRadius = 8
        clipsToBounds = false
        
        addTarget(self, action: #selector(pressed), for: [.touchDown, .touchDragEnter])
        addTarget(self, action: #selector(released), for: [.touchUpInside, .touchUpOutside, .touchCancel, .touchDragExit])
    }

    @objc private func pressed() {
        UIImpactFeedbackGenerator(style: .medium).impactOccurred()
        UIView.animate(withDuration: 0.1) {
            self.transform = CGAffineTransform(scaleX: 0.92, y: 0.92)
            self.configuration?.background.backgroundColor = UIColor.black.withAlphaComponent(0.7)
            self.configuration?.background.strokeColor = UIColor.white.withAlphaComponent(0.6)
        }
        if button != .exit {
            setVirtualButtonState(button.rawValue, true)
        }
    }

    @objc private func released() {
        UIImpactFeedbackGenerator(style: .light).impactOccurred()
        UIView.animate(withDuration: 0.1) {
            self.transform = .identity
            self.configuration?.background.backgroundColor = UIColor.black.withAlphaComponent(0.5)
            self.configuration?.background.strokeColor = UIColor.white.withAlphaComponent(0.3)
        }
        if button != .exit {
            setVirtualButtonState(button.rawValue, false)
        }
    }
    
    override func layoutSubviews() {
        super.layoutSubviews()
        configuration?.background.cornerRadius = bounds.height / 2
    }
}

final class VirtualGamepadView: UIView {
    private let baseButtonSize: CGFloat = 72
    private let baseSmallButtonSize: CGFloat = 54
    private let padding: CGFloat = 24
    private var exitButton: UIButton?
    private var dpadContainer: UIView?
    private var actionButtonsContainer: UIView?
    private var lastOrientationIsPortrait: Bool?
    private var exitButtonHeight: CGFloat = 0
    private var isDsMode: Bool = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        setupMode()
        setupView()
        rebuildControls()
        NotificationCenter.default.addObserver(self, selector: #selector(saveSlotChanged(_:)), name: Notification.Name.saveStateSlotChanged, object: nil)
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setupMode()
        setupView()
        rebuildControls()
        NotificationCenter.default.addObserver(self, selector: #selector(saveSlotChanged(_:)), name: Notification.Name.saveStateSlotChanged, object: nil)
    }

    private func setupMode() {
        if let state = currentEmuState {
            isDsMode = nes_is_ds(state) != 0
        }
    }

    deinit {
        NotificationCenter.default.removeObserver(self, name: Notification.Name.saveStateSlotChanged, object: nil)
        releaseVirtualButtons()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        superview?.bringSubviewToFront(self)

        let currentOrientation = UIScreen.main.bounds.height >= UIScreen.main.bounds.width
        if lastOrientationIsPortrait == nil {
            lastOrientationIsPortrait = currentOrientation
        } else if lastOrientationIsPortrait != currentOrientation {
            lastOrientationIsPortrait = currentOrientation
            rebuildControls()
        }
    }

    private func setupView() {
        backgroundColor = .clear
        isMultipleTouchEnabled = true
    }

    private func rebuildControls() {
        setupMode()
        exitButton = nil
        subviews.forEach { $0.removeFromSuperview() }
        lastOrientationIsPortrait = UIScreen.main.bounds.height >= UIScreen.main.bounds.width
        dpadContainer = nil
        actionButtonsContainer = nil
        createDPad()
        createActionButtons()
        createStartSelect()
        createExitButton()
        if isDsMode {
            createShoulderButtons()
        }
        setNeedsLayout()
    }

    private func createSaveStateControls() {}

    private func adaptiveSizes() -> (button: CGFloat, smallButton: CGFloat, isPortrait: Bool) {
        let screenBounds = UIScreen.main.bounds
        let isPortrait = screenBounds.height >= screenBounds.width
        let button = isPortrait ? baseButtonSize * 0.9 : baseButtonSize
        let small = isPortrait ? baseSmallButtonSize * 0.8 : baseSmallButtonSize
        return (button, small, isPortrait)
    }

    private func createDPad() {
        let container = UIView()
        container.translatesAutoresizingMaskIntoConstraints = false
        container.backgroundColor = .clear
        container.isUserInteractionEnabled = true
        addSubview(container)
        dpadContainer = container

        let sizes = adaptiveSizes()
        let buttonSize = sizes.button
        let offset = buttonSize * 0.85
        let span = offset * 2 + buttonSize

        let bottomPadding = sizes.isPortrait ? padding * 0.55 : padding
        let leadingPadding = sizes.isPortrait ? padding * 0.55 : padding

        NSLayoutConstraint.activate([
            container.leadingAnchor.constraint(equalTo: safeAreaLayoutGuide.leadingAnchor, constant: leadingPadding),
            container.bottomAnchor.constraint(equalTo: safeAreaLayoutGuide.bottomAnchor, constant: -bottomPadding),
            container.widthAnchor.constraint(equalToConstant: span),
            container.heightAnchor.constraint(equalToConstant: span)
        ])

        let upButton = VirtualGameButton(button: .up, title: "▲", size: buttonSize)
        let downButton = VirtualGameButton(button: .down, title: "▼", size: buttonSize)
        let leftButton = VirtualGameButton(button: .left, title: "◀", size: buttonSize)
        let rightButton = VirtualGameButton(button: .right, title: "▶", size: buttonSize)

        [upButton, downButton, leftButton, rightButton].forEach { button in
            container.addSubview(button)
            button.translatesAutoresizingMaskIntoConstraints = false
            button.widthAnchor.constraint(equalToConstant: buttonSize).isActive = true
            button.heightAnchor.constraint(equalToConstant: buttonSize).isActive = true
        }

        NSLayoutConstraint.activate([
            upButton.centerXAnchor.constraint(equalTo: container.centerXAnchor),
            upButton.centerYAnchor.constraint(equalTo: container.centerYAnchor, constant: -offset),

            downButton.centerXAnchor.constraint(equalTo: container.centerXAnchor),
            downButton.centerYAnchor.constraint(equalTo: container.centerYAnchor, constant: offset),

            leftButton.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            leftButton.centerXAnchor.constraint(equalTo: container.centerXAnchor, constant: -offset),

            rightButton.centerYAnchor.constraint(equalTo: container.centerYAnchor),
            rightButton.centerXAnchor.constraint(equalTo: container.centerXAnchor, constant: offset)
        ])
    }

    private func createActionButtons() {
        let container = UIView()
        container.translatesAutoresizingMaskIntoConstraints = false
        container.backgroundColor = .clear
        container.isUserInteractionEnabled = true
        addSubview(container)
        actionButtonsContainer = container

        let sizes = adaptiveSizes()
        let buttonSize = sizes.button
        let horizontalPadding = sizes.isPortrait ? padding * 0.7 : padding
        let bottomPadding = sizes.isPortrait ? padding * 0.6 : padding
        
        let widthMultiplier: CGFloat = isDsMode ? 3.2 : (sizes.isPortrait ? 2.4 : 2.2)
        let heightMultiplier: CGFloat = isDsMode ? 3.2 : (sizes.isPortrait ? 1.4 : 1.3)

        var constraints: [NSLayoutConstraint] = [
            container.trailingAnchor.constraint(equalTo: safeAreaLayoutGuide.trailingAnchor, constant: -horizontalPadding),
            container.widthAnchor.constraint(equalToConstant: buttonSize * widthMultiplier),
            container.heightAnchor.constraint(equalToConstant: buttonSize * heightMultiplier)
        ]

        if sizes.isPortrait, let dpad = dpadContainer {
            constraints.append(container.centerYAnchor.constraint(equalTo: dpad.centerYAnchor))
        } else {
            constraints.append(container.bottomAnchor.constraint(equalTo: safeAreaLayoutGuide.bottomAnchor, constant: -bottomPadding))
        }

        NSLayoutConstraint.activate(constraints)

        let buttonB = VirtualGameButton(button: .b, title: "B", size: buttonSize)
        let buttonA = VirtualGameButton(button: .a, title: "A", size: buttonSize)

        let offset = buttonSize * 0.85
        
        if isDsMode {
            let buttonX = VirtualGameButton(button: .x, title: "X", size: buttonSize)
            let buttonY = VirtualGameButton(button: .y, title: "Y", size: buttonSize)
            
            [buttonA, buttonB, buttonX, buttonY].forEach {
                container.addSubview($0)
                $0.translatesAutoresizingMaskIntoConstraints = false
                $0.widthAnchor.constraint(equalToConstant: buttonSize).isActive = true
                $0.heightAnchor.constraint(equalToConstant: buttonSize).isActive = true
            }

            NSLayoutConstraint.activate([
                buttonA.centerYAnchor.constraint(equalTo: container.centerYAnchor),
                buttonA.trailingAnchor.constraint(equalTo: container.trailingAnchor, constant: -offset * 0.2),
                
                buttonY.centerYAnchor.constraint(equalTo: container.centerYAnchor),
                buttonY.leadingAnchor.constraint(equalTo: container.leadingAnchor, constant: offset * 0.2),
                
                buttonX.centerXAnchor.constraint(equalTo: container.centerXAnchor),
                buttonX.topAnchor.constraint(equalTo: container.topAnchor, constant: offset * 0.2),
                
                buttonB.centerXAnchor.constraint(equalTo: container.centerXAnchor),
                buttonB.bottomAnchor.constraint(equalTo: container.bottomAnchor, constant: -offset * 0.2)
            ])
        } else {
            [buttonA, buttonB].forEach {
                container.addSubview($0)
                $0.translatesAutoresizingMaskIntoConstraints = false
                $0.widthAnchor.constraint(equalToConstant: buttonSize).isActive = true
                $0.heightAnchor.constraint(equalToConstant: buttonSize).isActive = true
            }

            NSLayoutConstraint.activate([
                buttonB.centerYAnchor.constraint(equalTo: container.centerYAnchor),
                buttonB.leadingAnchor.constraint(equalTo: container.leadingAnchor),

                buttonA.centerYAnchor.constraint(equalTo: container.centerYAnchor),
                buttonA.trailingAnchor.constraint(equalTo: container.trailingAnchor)
            ])
        }
    }

    private func createStartSelect() {
        let sizes = adaptiveSizes()
        let smallButtonSize = sizes.smallButton

        let selectButton = VirtualGameButton(button: .select, title: "Select", size: smallButtonSize)
        let startButton = VirtualGameButton(button: .start, title: "Start", size: smallButtonSize)

        selectButton.translatesAutoresizingMaskIntoConstraints = false
        startButton.translatesAutoresizingMaskIntoConstraints = false

        let widthFactor: CGFloat = sizes.isPortrait ? 2.4 : 1.8
        let heightFactor: CGFloat = sizes.isPortrait ? 0.8 : 0.7
        [selectButton, startButton].forEach { button in
            button.widthAnchor.constraint(equalToConstant: smallButtonSize * widthFactor).isActive = true
            button.heightAnchor.constraint(equalToConstant: smallButtonSize * heightFactor).isActive = true
            button.layer.cornerRadius = smallButtonSize * heightFactor * 0.5
        }
        
        let inset = sizes.isPortrait ? CGSize(width: 16, height: 8) : CGSize(width: 14, height: 8)
        let insets = NSDirectionalEdgeInsets(top: inset.height, leading: inset.width, bottom: inset.height, trailing: inset.width)
        
        selectButton.configuration?.contentInsets = insets
        startButton.configuration?.contentInsets = insets

        let verticalSpacing = padding * 0.45
        if sizes.isPortrait {
            addSubview(selectButton)
            addSubview(startButton)
            let dpadTop = dpadContainer?.topAnchor ?? safeAreaLayoutGuide.bottomAnchor
            NSLayoutConstraint.activate([
                selectButton.leadingAnchor.constraint(equalTo: safeAreaLayoutGuide.leadingAnchor, constant: padding * 0.6),
                selectButton.bottomAnchor.constraint(equalTo: dpadTop, constant: -verticalSpacing),

                startButton.trailingAnchor.constraint(equalTo: safeAreaLayoutGuide.trailingAnchor, constant: -padding * 0.6),
                startButton.bottomAnchor.constraint(equalTo: selectButton.bottomAnchor)
            ])
        } else {
            let stack = UIStackView(arrangedSubviews: [selectButton, startButton])
            stack.translatesAutoresizingMaskIntoConstraints = false
            stack.axis = .horizontal
            stack.distribution = .equalSpacing
            stack.alignment = .center
            stack.spacing = padding * 0.8
            addSubview(stack)

            NSLayoutConstraint.activate([
                stack.centerXAnchor.constraint(equalTo: centerXAnchor),
                stack.bottomAnchor.constraint(equalTo: safeAreaLayoutGuide.bottomAnchor, constant: -(padding * 1.4))
            ])
        }
    }

    private func createShoulderButtons() {
        let sizes = adaptiveSizes()
        let buttonSize = sizes.button
        
        let lButton = VirtualGameButton(button: .l, title: "L", size: buttonSize)
        let rButton = VirtualGameButton(button: .r, title: "R", size: buttonSize)
        
        [lButton, rButton].forEach { button in
            addSubview(button)
            button.translatesAutoresizingMaskIntoConstraints = false
            button.widthAnchor.constraint(equalToConstant: buttonSize).isActive = true
            button.heightAnchor.constraint(equalToConstant: buttonSize).isActive = true
        }
        
        NSLayoutConstraint.activate([
            lButton.leadingAnchor.constraint(equalTo: safeAreaLayoutGuide.leadingAnchor, constant: padding * 0.6),
            lButton.bottomAnchor.constraint(equalTo: dpadContainer!.topAnchor, constant: -padding * 0.4),
            
            rButton.trailingAnchor.constraint(equalTo: safeAreaLayoutGuide.trailingAnchor, constant: -padding * 0.6),
            rButton.bottomAnchor.constraint(equalTo: actionButtonsContainer!.topAnchor, constant: -padding * 0.4)
        ])
    }

    private func createExitButton() {
        guard exitButton == nil else { return }
        let sizes = adaptiveSizes()
        let button = UIButton(type: .system)
        button.translatesAutoresizingMaskIntoConstraints = false
        
        let capsuleHeight = sizes.smallButton * (sizes.isPortrait ? 0.9 : 0.8)
        exitButtonHeight = capsuleHeight

        var config = UIButton.Configuration.plain()
        config.contentInsets = NSDirectionalEdgeInsets(top: 8, leading: 18, bottom: 8, trailing: 18)
        config.baseForegroundColor = .white
        
        config.background.backgroundColor = UIColor.black.withAlphaComponent(0.5)
        config.background.cornerRadius = capsuleHeight / 2
        config.background.strokeColor = UIColor.white.withAlphaComponent(0.3)
        config.background.strokeWidth = 1.0
        
        var container = AttributeContainer()
        container.font = UIFont.systemFont(ofSize: 12, weight: .semibold)
        config.attributedTitle = AttributedString("Menu", attributes: container)
        
        button.configuration = config
        addSubview(button)

        let topPadding = sizes.isPortrait ? padding * 0.8 : padding
        let trailingPadding = sizes.isPortrait ? padding * 0.8 : padding

        NSLayoutConstraint.activate([
            button.heightAnchor.constraint(equalToConstant: capsuleHeight),
            button.topAnchor.constraint(equalTo: safeAreaLayoutGuide.topAnchor, constant: topPadding),
            button.trailingAnchor.constraint(equalTo: safeAreaLayoutGuide.trailingAnchor, constant: -trailingPadding)
        ])

        exitButton = button
        updateSlotSelectionHighlight()
        
        // Native UIMenu
        button.showsMenuAsPrimaryAction = true
        button.menu = createMainMenu()
    }

    private func createMainMenu() -> UIMenu {
        let bridge = SaveStateBridge.shared
        
        let deferred = UIDeferredMenuElement.uncached { [weak self] completion in
            if bridge.pauseOnMenu {
                bridge.menuOverlayVisibilityChanged(true)
            }
            
            let resumeAction = UIAction(title: "Resume Game", image: UIImage(systemName: "play.fill")) { _ in
                bridge.menuOverlayVisibilityChanged(false)
            }
            
            let slotsMenu = UIMenu(title: "Save Slots", options: .displayInline, children: (0..<bridge.slotCount).map { slot in
                UIAction(title: "Slot \(slot + 1)", state: bridge.currentSlot == slot ? .on : .off) { [weak self] _ in
                    SaveStateBridge.shared.setCurrentSlot(slot)
                    self?.updateSlotSelectionHighlight()
                    // Re-create menu to update state (will apply next time menu is opened)
                    self?.exitButton?.menu = self?.createMainMenu()
                }
            })
            
            let stateActions = UIMenu(title: "State", options: .displayInline, children: [
                UIAction(title: "Save State", image: UIImage(systemName: "square.and.arrow.down")) { _ in
                    SaveStateBridge.shared.quickSave()
                    // Auto-resume on action?
                    if bridge.pauseOnMenu { bridge.menuOverlayVisibilityChanged(false) }
                },
                UIAction(title: "Load State", image: UIImage(systemName: "square.and.arrow.up")) { _ in
                    SaveStateBridge.shared.quickLoad()
                    if bridge.pauseOnMenu { bridge.menuOverlayVisibilityChanged(false) }
                }
            ])
            
            let optionsMenu = UIMenu(title: "Options", options: .displayInline, children: [
                UIAction(title: "Auto Save", state: bridge.autoSaveEnabled ? .on : .off) { [weak self] _ in
                    SaveStateBridge.shared.toggleAutoSave()
                    self?.exitButton?.menu = self?.createMainMenu()
                },
                UIAction(title: "Pause on Menu", state: bridge.pauseOnMenu ? .on : .off) { [weak self] _ in
                    SaveStateBridge.shared.togglePauseOnMenu()
                    self?.exitButton?.menu = self?.createMainMenu()
                }
            ])
            
            let sessionMenu = UIMenu(title: "Session", options: .displayInline, children: [
                UIAction(title: "Reset Game", image: UIImage(systemName: "arrow.counterclockwise"), attributes: .destructive) { [weak self] _ in
                    self?.presentNativeAlert(title: "Reset Game", message: "Are you sure you want to reset? Unsaved progress will be lost.", actionTitle: "Reset") {
                        setVirtualButtonState(8, true)
                        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
                            setVirtualButtonState(8, false)
                        }
                        if bridge.pauseOnMenu { bridge.menuOverlayVisibilityChanged(false) }
                    }
                },
                UIAction(title: "Exit Game", image: UIImage(systemName: "xmark.circle"), attributes: .destructive) { [weak self] _ in
                    self?.presentNativeAlert(title: "Exit Game", message: "Are you sure you want to quit? Unsaved progress will be lost.", actionTitle: "Exit") {
                        setVirtualButtonState(9, true)
                        // No need to unpause as we exit
                    }
                }
            ])
            
            var items: [UIMenuElement] = []
            if bridge.pauseOnMenu {
                items.append(resumeAction)
            }
            items.append(contentsOf: [slotsMenu, stateActions, optionsMenu, sessionMenu])
            
            completion(items)
        }
        
        return UIMenu(title: "Menu", children: [deferred])
    }

    private func presentNativeAlert(title: String, message: String, actionTitle: String, actionHandler: @escaping () -> Void) {
        // Find the topmost view controller to present the alert
        guard let window = self.window,
              let rootVC = window.rootViewController else { return }

        let alert = UIAlertController(title: title, message: message, preferredStyle: .alert)
        
        let confirmAction = UIAlertAction(title: actionTitle, style: .destructive) { _ in
            actionHandler()
        }
        let cancelAction = UIAlertAction(title: "Cancel", style: .cancel, handler: nil)
        
        alert.addAction(confirmAction)
        alert.addAction(cancelAction)
        
        // Present on rootVC (or presented VC if one exists)
        var presenter = rootVC
        while let presented = presenter.presentedViewController {
            presenter = presented
        }
        
        // Slight delay to ensure menu dismissal doesn't conflict
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            presenter.present(alert, animated: true)
        }
    }

    @objc private func saveSlotChanged(_ notification: Notification) {
        updateSlotSelectionHighlight()
        exitButton?.menu = createMainMenu()
    }

    private func updateSlotSelectionHighlight() {
        let current = SaveStateBridge.shared.currentSlot
        let title = "Menu · Slot \(current + 1)"
        
        if var config = exitButton?.configuration {
            var container = AttributeContainer()
            container.font = UIFont.systemFont(ofSize: 12, weight: .semibold)
            config.attributedTitle = AttributedString(title, attributes: container)
            exitButton?.configuration = config
        } else {
             exitButton?.setTitle(title, for: .normal)
        }
    }

    // MARK: - Touch handling

    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
        let view = super.hitTest(point, with: event)
        // If we hit a button, return the button.
        // Otherwise, return self to ensure touchesBegan is called for touchscreen processing.
        if view is VirtualGameButton || view is UIButton {
            return view
        }
        return self
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        handleTouch(touches, pressed: true)
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        handleTouch(touches, pressed: true)
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        handleTouch(touches, pressed: false)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        handleTouch(touches, pressed: false)
    }

    private func handleTouch(_ touches: Set<UITouch>, pressed: Bool) {
        guard let state = currentEmuState else { return }
        guard let touch = touches.first else { return }
        
        let location = touch.location(in: self)
        let rect = getEmulatorScreenRect()
        
        // If the touch is inside the emulator screen area
        if rect.contains(location) {
            // Translate to 256x384 logical coordinates
            let x = (location.x - rect.origin.x) * (256.0 / rect.width)
            let y = (location.y - rect.origin.y) * (384.0 / rect.height)
            
            nes_touch(state, Int32(x), Int32(y), pressed ? 1 : 0)
        } else if !pressed {
            // Ensure touch ends if we release outside
            nes_touch(state, 0, 0, 0)
        }
    }

    private func getEmulatorScreenRect() -> CGRect {
        let bounds = self.bounds
        let logicalWidth: CGFloat = 256
        let logicalHeight: CGFloat = 384
        
        let scale = min(bounds.width / logicalWidth, bounds.height / logicalHeight)
        let w = logicalWidth * scale
        let h = logicalHeight * scale
        let x = (bounds.width - w) / 2
        let y = (bounds.height - h) / 2
        
        return CGRect(x: x, y: y, width: w, height: h)
    }
}

// MARK: - ContentView

struct ContentView: View {
    @StateObject private var library = GameLibrary()
    @State private var emulatorRunning = false
    @State private var showingImporter = false
    @State private var alertInfo: AlertInfo?

    init() {
        // SDL initialization must be prepared on the main thread
        SDL_SetMainReady()
        // Disable SDL event pump since we handle input via SwiftUI
        SDL_iPhoneSetEventPump(SDL_FALSE) 
    }

    var body: some View {
        GeometryReader { proxy in
            ZStack {
                if emulatorRunning {
                    Color.black.opacity(0.9).ignoresSafeArea()
                } else {
                    HomeScreen(
                        library: library,
                        startGame: handleGameSelection,
                        addGame: { showingImporter = true },
                        removeGame: { library.remove($0) },
                        isPortrait: proxy.size.height > proxy.size.width,
                        coverProvider: { library.coverImage(for: $0) }
                    )
                    .ignoresSafeArea(.container, edges: .bottom)
                    .transition(.opacity)
                }
            }
            .background(Color.black.opacity(0.95).ignoresSafeArea())
        }
        .fileImporter(isPresented: $showingImporter, allowedContentTypes: ContentView.supportedTypes, allowsMultipleSelection: true) { result in
            switch result {
                case .success(let urls):
                    var lastError: Error?
                    for url in urls {
                        do {
                            try library.addGame(from: url)
                        } catch {
                            lastError = error
                        }
                    }
                    if let error = lastError {
                        alertInfo = AlertInfo(title: "Unable to Add Game", message: error.localizedDescription)
                    }

                case .failure(let error):
                    alertInfo = AlertInfo(title: "Import Failed", message: error.localizedDescription)
            }
        }
        .alert(item: $alertInfo) { info in
            Alert(title: Text(info.title), message: Text(info.message), dismissButton: .default(Text("OK")))
        }
    }

    private func handleGameSelection(_ entry: GameEntry) {
        UserDefaults.standard.set(true, forKey: "isVirtualController")

        do {
            let url = try library.resolveURL(for: entry)
            let bridge = SaveStateBridge.shared
            let totalSlots = 4
            bridge.configure(slotCount: totalSlots)
            let baseStateURL = library.saveStateBaseURL(for: entry)
            bridge.setCurrentSlot(bridge.currentSlot, notify: true)
            
            sstartEmu(romURL: url, stateBaseURL: baseStateURL, slotCount: totalSlots, initialSlot: bridge.currentSlot)
        } catch {
            alertInfo = AlertInfo(
                title: "Launch Failed",
                message: "Could not open \(entry.name). Please re-add this game."
            )
            library.remove(entry)
        }
    }

    private func sstartEmu(romURL: URL, stateBaseURL: URL, slotCount: Int, initialSlot: Int) {
        removeGamepadOverlay()
        patchMakeKeyAndVisible()
        let romPath = romURL.path
        emulatorRunning = true
        
        guard let emuState = initEmu(romPath) else {
            print("Failed to initialize emulator")
            self.emulatorRunning = false
            return
        }
        currentEmuState = emuState
        
        let displayLink = CADisplayLink(target: DisplayLinkProxy(callback: {
            renderFrame(emuState)
        }), selector: #selector(DisplayLinkProxy.onDisplayLink))
        displayLink.add(to: .main, forMode: .common)
        
        DispatchQueue.global(qos: .userInteractive).async {
            runEmuLoop(emuState)
            
            DispatchQueue.main.async {
                displayLink.invalidate()
                cleanupEmu(emuState)
                currentEmuState = nil
                
                removeGamepadOverlay()
                self.emulatorRunning = false
            }
            
            releaseVirtualButtons()
        }
    }

    private static var supportedTypes: [UTType] {
        if let nes = UTType(filenameExtension: "nes"), 
           let nds = UTType(filenameExtension: "nds"),
           let zip = UTType(filenameExtension: "zip") {
            return [nes, nds, zip]
        }
        return [.data]
    }
}

class DisplayLinkProxy {
    let callback: () -> Void
    
    init(callback: @escaping () -> Void) {
        self.callback = callback
    }
    
    @objc func onDisplayLink() {
        callback()
    }
}

#Preview {
    ContentView()
}

// MARK: - UIKit hooks

func patchMakeKeyAndVisible() {
    struct SwizzleState {
        static var hasSwizzled = false
    }

    guard !SwizzleState.hasSwizzled else { return }

    let uiwindowClass = UIWindow.self
    if let m1 = class_getInstanceMethod(uiwindowClass, #selector(UIWindow.makeKeyAndVisible)),
       let m2 = class_getInstanceMethod(uiwindowClass, #selector(UIWindow.wdb_makeKeyAndVisible)) {
        method_exchangeImplementations(m1, m2)
        SwizzleState.hasSwizzled = true
    }
}

var theWindow: UIWindow? = nil
extension UIWindow {
    @objc func wdb_makeKeyAndVisible() {
        if #available(iOS 13.0, *) {
            self.windowScene = (UIApplication.shared.connectedScenes.first! as! UIWindowScene)
        }
        self.wdb_makeKeyAndVisible()
        theWindow = self

        if UserDefaults.standard.bool(forKey: "isVirtualController"), theWindow != nil {
            waitforcontroller()
        }
    }
}

func waitforcontroller() {
    guard let window = theWindow else { return }

    Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { timer in
        DispatchQueue.main.async {
            guard window.viewWithTag(9001) == nil else {
                timer.invalidate()
                return
            }

            guard !window.subviews.isEmpty else {
                return
            }

            let overlay = VirtualGamepadView()
            overlay.translatesAutoresizingMaskIntoConstraints = false
            overlay.tag = 9001
            overlay.isUserInteractionEnabled = true
            overlay.isMultipleTouchEnabled = true
            
            if let rootVC = window.rootViewController {
                rootVC.view.addSubview(overlay)
                NSLayoutConstraint.activate([
                    overlay.leadingAnchor.constraint(equalTo: rootVC.view.leadingAnchor),
                    overlay.trailingAnchor.constraint(equalTo: rootVC.view.trailingAnchor),
                    overlay.topAnchor.constraint(equalTo: rootVC.view.topAnchor),
                    overlay.bottomAnchor.constraint(equalTo: rootVC.view.bottomAnchor)
                ])
                rootVC.view.bringSubviewToFront(overlay)
            } else {
                window.addSubview(overlay)
                NSLayoutConstraint.activate([
                    overlay.leadingAnchor.constraint(equalTo: window.leadingAnchor),
                    overlay.trailingAnchor.constraint(equalTo: window.trailingAnchor),
                    overlay.topAnchor.constraint(equalTo: window.topAnchor),
                    overlay.bottomAnchor.constraint(equalTo: window.bottomAnchor)
                ])
                window.bringSubviewToFront(overlay)
            }

            timer.invalidate()
        }
    }
}

func removeGamepadOverlay() {
    theWindow?.viewWithTag(9001)?.removeFromSuperview()
}

// MARK: - HomeScreen

struct HomeScreen: View {
    @ObservedObject var library: GameLibrary
    let startGame: (GameEntry) -> Void
    let addGame: () -> Void
    let removeGame: (GameEntry) -> Void
    let isPortrait: Bool
    let coverProvider: (GameEntry) -> UIImage?

    private var columns: [GridItem] {
        if isPortrait {
            return [GridItem(.flexible(minimum: 140), spacing: 16), GridItem(.flexible(minimum: 140), spacing: 16)]
        } else {
            return [GridItem(.adaptive(minimum: 220), spacing: 24)]
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: isPortrait ? 18 : 24) {
            if isPortrait {
                VStack(alignment: .leading, spacing: 16) {
                    header
                    Button(action: addGame) {
                        Label("Add Game", systemImage: "plus")
                            .font(.system(size: 16, weight: .semibold))
                    }
                    .buttonStyle(OverlayButtonStyle())
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            } else {
                HStack {
                    header
                    Spacer()
                    Button(action: addGame) {
                        Label("Add Game", systemImage: "plus")
                            .font(.system(size: 16, weight: .semibold))
                    }
                    .buttonStyle(OverlayButtonStyle())
                }
            }

            if library.games.isEmpty {
                emptyState
            } else {
                ScrollView(showsIndicators: false) {
                    LazyVStack(alignment: .leading, spacing: isPortrait ? 32 : 40) {
                        ForEach(platformSections) { section in
                            VStack(alignment: .leading, spacing: isPortrait ? 16 : 20) {
                                HStack(spacing: 8) {
                                    Text(section.platform.displayName)
                                        .font(.system(size: isPortrait ? 22 : 24, weight: .bold))
                                        .foregroundColor(.white)
                                    if section.platform.isBeta {
                                        BetaTag()
                                    }
                                }
                                .padding(.horizontal, 4)

                                LazyVGrid(columns: columns, spacing: isPortrait ? 16 : 24) {
                                    ForEach(section.games) { game in
                                        Button {
                                            startGame(game)
                                        } label: {
                                            GameCoverView(game: game, coverImage: coverProvider(game), aspectRatio: section.platform.coverAspectRatio)
                                        }
                                        .buttonStyle(GameTileButtonStyle())
                                        .contextMenu {
                                            Button(role: .destructive) {
                                                removeGame(game)
                                            } label: {
                                                Label("Remove", systemImage: "trash")
                                            }
                                        }
                                    }
                                }
                            }
                            .padding(.top, 4)
                        }
                    }
                    .padding(.top, 8)
                }
                .ignoresSafeArea(edges: .bottom)
            }
        }
        .padding(EdgeInsets(top: 24, leading: isPortrait ? 16 : 24, bottom: 24, trailing: isPortrait ? 16 : 24))
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("StikEMU")
                .font(.system(size: 36, weight: .bold))
                .foregroundColor(.white)
            Text("Select a game to start playing")
                .font(.system(size: 16, weight: .medium))
                .foregroundColor(Color.white.opacity(0.7))
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "gamecontroller.fill")
                .font(.system(size: 48))
                .foregroundColor(Color.white.opacity(0.6))
            Text("No games yet")
                .font(.system(size: 18, weight: .semibold))
                .foregroundColor(.white)
            Text("Tap Add Game to import a .nes file into the Nintendo Entertainment System library.")
                .foregroundColor(Color.white.opacity(0.7))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var platformSections: [PlatformSection] {
        let grouped = Dictionary(grouping: library.games) { $0.platform }
        return GamePlatform.allCases.compactMap { platform in
            guard let games = grouped[platform], !games.isEmpty else { return nil }
            return PlatformSection(platform: platform, games: games)
        }
    }

    private struct PlatformSection: Identifiable {
        let platform: GamePlatform
        let games: [GameEntry]

        var id: GamePlatform { platform }
    }

    private struct BetaTag: View {
        var body: some View {
            Text("BETA")
                .font(.system(size: 12, weight: .semibold))
                .foregroundColor(Color.white.opacity(0.85))
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(
                    Capsule()
                        .fill(Color.orange.opacity(0.65))
                )
        }
    }
}

struct GameTileButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.97 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
            .contentShape(RoundedRectangle(cornerRadius: 24))
    }
}

struct OverlayButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(
                Capsule()
                    .fill(Color.black.opacity(0.45))
                    .overlay(
                        Capsule()
                            .stroke(Color.white.opacity(0.25), lineWidth: 1)
                    )
            )
            .foregroundStyle(Color.white)
            .scaleEffect(configuration.isPressed ? 0.95 : 1.0)
            .animation(.easeOut(duration: 0.15), value: configuration.isPressed)
            .clipShape(Capsule())
            .contentShape(Capsule())
    }
}

struct GameCoverView: View {
    let game: GameEntry
    let coverImage: UIImage?
    let aspectRatio: CGFloat

    var body: some View {
        ZStack(alignment: .bottomLeading) {
            coverLayer
            VStack(alignment: .leading, spacing: 4) {
                Text(game.name)
                    .font(.system(size: 18, weight: .semibold))
                    .foregroundColor(.white)
                    .lineLimit(2)
                Text("Tap to play")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundColor(Color.white.opacity(0.75))
            }
            .padding(14)
        }
        .aspectRatio(aspectRatio, contentMode: .fit)
        .clipShape(RoundedRectangle(cornerRadius: 24))
    }

    @ViewBuilder
    private var coverLayer: some View {
        let overlayGradient = LinearGradient(colors: [.black.opacity(0.65), .black.opacity(0.2)], startPoint: .bottom, endPoint: .top)

        if let image = coverImage {
            Image(uiImage: image)
                .resizable()
                .scaledToFill()
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .overlay(overlayGradient)
                .clipShape(RoundedRectangle(cornerRadius: 20))
                .overlay(
                    RoundedRectangle(cornerRadius: 20)
                        .stroke(Color.white.opacity(0.12), lineWidth: 1)
                )
        } else {
            RoundedRectangle(cornerRadius: 20)
                .fill(LinearGradient(colors: gradientColors(for: game), startPoint: .topLeading, endPoint: .bottomTrailing))
                .overlay(overlayGradient)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .overlay(
                    RoundedRectangle(cornerRadius: 20)
                        .stroke(Color.white.opacity(0.15), lineWidth: 1)
                )
                .overlay(
                    Text(String(game.name.prefix(1)).uppercased())
                        .font(.system(size: 52, weight: .heavy))
                        .foregroundColor(Color.white.opacity(0.25))
                        .padding(.top, 12)
                        .padding(.leading, 16),
                    alignment: .topLeading
                )
        }
    }

    private func gradientColors(for game: GameEntry) -> [Color] {
        let hash = abs(game.id.uuidString.hashValue)
        let hue1 = Double((hash & 0xFF)) / 255.0
        let hue2 = Double(((hash >> 8) & 0xFF)) / 255.0
        let color1 = Color(hue: hue1, saturation: 0.65, brightness: 0.85)
        let color2 = Color(hue: hue2, saturation: 0.75, brightness: 0.75)
        return [color1, color2]
    }
}