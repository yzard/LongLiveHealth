import Testing
@testable import LongLiveHealthCore

@Test func browseAcceptsNewServerCategoriesWithoutClientRelease() {
    let category = BrowseCategory(.object(["id": .string("new-category"), "title": .string("New category"), "symbol": .string("star"), "color": .string("pink"), "section": .string("New section")]))
    #expect(category.id == "new-category" && category.title == "New category")
    #expect(category.section == "New section" && category.symbol == "star")
}
