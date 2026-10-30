import Testing
@testable import LongLiveHealthCore

@Test func metricEducationPreservesClinicalDistinctions() {
    let pairs = [
        ("Albumin", "Albumin, Random Urine without Creatinine"),
        ("Thyroglobulin", "Thyroglobulin Antibodies"),
        ("Absolute Basophil Count", "Basophil Automated"),
        ("Magnesium", "Magnesium, RBC"),
        ("Vitamin D", "Vitamin D,25-OH, Total, IA"),
        ("Testosterone, Free", "Testosterone, Total"),
        ("ABORh", "AB Screen"),
        ("Q-T Interval", "QTc"),
        ("P Axis", "P wave Atrium by EKG"),
        ("QRS Duration", "QRS complex Ventricles by EKG")
    ]
    for (left, right) in pairs {
        let first = MetricEducation.explanation(id: "clinical:unknown", name: left)
        let second = MetricEducation.explanation(id: "clinical:unknown", name: right)
        #expect(first != nil && second != nil)
        #expect(first != second)
    }
    #expect(MetricEducation.explanation(id: "clinical:new", name: "Unknown albumin antibody") == nil)
    #expect(MetricEducation.explanation(id: "clinical:new", name: "Glucose in cerebrospinal fluid") == nil)
    #expect(MetricEducation.explanation(id: "clinical:new", name: "calcium") == MetricEducation.explanation(id: "calcium", name: ""))
    #expect(MetricEducation.explanation(id: "clinical:new", name: "Thyroglobulin Ab [Units/volume] in Serum") == MetricEducation.explanation(id: "thyroglobulin_antibodies", name: ""))
}
