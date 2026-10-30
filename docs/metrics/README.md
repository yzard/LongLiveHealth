# Browse metric education — Build 48

Reviewed 2026-10-22. All current laboratory definitions, daily metrics, measurement kinds and observed hospital/report names have specific explanations. This is an editorial coverage audit, not a clinician sign-off.

The catalog contains 169 explanations; 290 observed names resolve through explicit aliases. No record values or patient identifiers are included in this audit.

`education.tsv` is the editable content. `education-aliases.json` only maps exact normalized names; it does not change medical record identity or merge results. Unknown future tests show an explicit unavailable-description state rather than borrowing a similar test's meaning.

Regenerate with `python3 src/tools/generate_metric_education.py`. Validate with `python3 src/tools/generate_metric_education.py --check` and `python3 -m unittest discover -s tests/tools -v`.

## Individual explanations and references

| Metric | Explanation | Reference |
| --- | --- | --- |
| heart_rate | Heart rate is the number of heartbeats per minute. It reflects how quickly the heart delivers blood and oxygen; activity, sleep, stress, illness and medicines can change it. | [Reference](https://www.heart.org/en/health-topics/high-blood-pressure/the-facts-about-high-blood-pressure/all-about-heart-rate-pulse) |
| resting_heart_rate | Resting heart rate describes how fast the heart beats while you are at rest. Tracking your usual pattern can help show changes in fitness or recovery; illness, dehydration and medicines can also alter it. | [Reference](https://www.heart.org/en/health-topics/high-blood-pressure/the-facts-about-high-blood-pressure/all-about-heart-rate-pulse) |
| hrv_sdnn | Heart rate variability describes changes in the time between normal heartbeats. SDNN summarizes that variation in milliseconds and relates to autonomic nervous system activity. Recording length, breathing and measurement conditions affect comparisons. | [Reference](https://occup-med.biomedcentral.com/articles/10.1186/s12995-024-00414-9) |
| vo2_max | VO₂ max describes the greatest amount of oxygen your body can use during intense exercise, relative to body weight. It reflects the combined capacity of the heart, lungs and muscles for aerobic exercise; a watch estimates it rather than measuring it directly. | [Reference](https://support.apple.com/en-us/108790) |
| sleep | Sleep duration is time spent asleep. Sleep supports memory, emotional regulation, immune function and metabolic health. Duration is useful alongside sleep quality, regularity and how rested you feel. | [Reference](https://www.nhlbi.nih.gov/health/sleep/why-sleep-important) |
| sleep_sessions | A sleep session describes a period of sleep, including overnight sleep and naps. Timing, interruptions and sleep stages help describe sleep patterns that relate to alertness, recovery and daily functioning. | [Reference](https://www.nhlbi.nih.gov/health/sleep/why-sleep-important) |
| workout | Workout duration measures time spent in recorded exercise sessions. Regular activity supports cardiovascular fitness, muscle strength and metabolic health; the type and intensity of exercise matter as well as its duration. | [Reference](https://www.nhlbi.nih.gov/health/heart/physical-activity/benefits) |
| steps | Steps count walking and running movements recorded throughout the day. They describe everyday movement and can help track activity habits; step totals do not capture all exercise, such as cycling or swimming. | [Reference](https://www.nhlbi.nih.gov/health/heart/physical-activity/benefits) |
| active_energy | Active energy estimates calories used through movement and exercise above resting needs. It helps describe activity-related energy expenditure; device estimates depend on the activity, sensors and personal information. | [Reference](https://www.niddk.nih.gov/health-information/weight-management/adult-overweight-obesity/factors-affecting-weight-health) |
| basal_energy | Resting energy estimates calories needed to maintain essential functions such as breathing, circulation and temperature regulation. It makes up part of total daily energy expenditure and varies with body size, body composition and age. | [Reference](https://www.niddk.nih.gov/health-information/weight-management/adult-overweight-obesity/factors-affecting-weight-health) |
| body_mass | Weight measures total body mass, including fat, muscle, bone and body water. Longer-term changes can reflect nutrition and health, while day-to-day shifts often reflect fluid balance, food and measurement timing. | [Reference](https://www.niddk.nih.gov/health-information/weight-management/adult-overweight-obesity/definition-facts) |
| body_fat | Body fat percentage estimates how much of body weight is fat tissue. Fat stores energy and supports hormone function; its amount and distribution relate to metabolic health. Hydration and measurement method affect estimates. | [Reference](https://www.niddk.nih.gov/health-information/weight-management/adult-overweight-obesity/definition-facts) |
| oxygen_saturation | Oxygen saturation estimates the percentage of hemoglobin carrying oxygen. It helps describe oxygen delivery from the lungs to the body. Motion, circulation and sensor fit can affect a wearable reading. | [Reference](https://medlineplus.gov/lab-tests/pulse-oximetry/) |
| respiratory_rate | Respiratory rate is the number of breaths per minute. It reflects breathing activity and can change with exercise, sleep, fever or respiratory illness; compare readings taken under similar conditions. | [Reference](https://medlineplus.gov/vitalsigns.html) |
| blood_glucose | Blood glucose measures sugar circulating in the blood, a major energy source for cells. It helps assess glucose regulation and diabetes; meals, activity, medicines and illness influence the value. | [Reference](https://medlineplus.gov/lab-tests/blood-glucose-test/) |
| glucose | Glucose is the main sugar circulating in the blood and a source of energy for the body. Blood testing helps assess diabetes and glucose regulation; fasting and after-meal results have different meanings. | [Reference](https://medlineplus.gov/lab-tests/blood-glucose-test/) |
| hba1c | HbA1c measures the proportion of hemoglobin with glucose attached and reflects average blood glucose over roughly the past three months. It is used to assess diabetes and longer-term glucose control; altered red-cell lifespan can affect it. | [Reference](https://medlineplus.gov/lab-tests/hemoglobin-a1c-hba1c-test/) |
| total_cholesterol | Total cholesterol measures cholesterol carried by all major lipoproteins in the blood. Cholesterol is needed for cells and hormones; this result is interpreted with LDL, HDL and other cardiovascular risk factors. | [Reference](https://medlineplus.gov/lab-tests/cholesterol-levels/) |
| ldl_cholesterol | LDL cholesterol measures cholesterol carried in low-density lipoproteins. Excess LDL can contribute to plaque in artery walls, so it is an important marker for cardiovascular risk and cholesterol-lowering treatment. | [Reference](https://medlineplus.gov/lab-tests/cholesterol-levels/) |
| hdl_cholesterol | HDL cholesterol measures cholesterol in high-density lipoproteins, which participate in returning cholesterol to the liver. It contributes to cardiovascular risk assessment but does not by itself measure how well HDL functions. | [Reference](https://medlineplus.gov/lab-tests/cholesterol-levels/) |
| triglycerides | Triglycerides are fats used to store and transport energy. Raised levels can accompany metabolic problems; very high levels increase pancreatitis risk. Meals and alcohol can influence the result. | [Reference](https://medlineplus.gov/lab-tests/triglycerides-test/) |
| apob | Apolipoprotein B is a structural protein on lipoprotein particles that can deposit cholesterol in artery walls. Its concentration helps assess the burden of these particles and cardiovascular risk, especially when cholesterol measures disagree. | [Reference](https://www.mayocliniclabs.com/test-catalog/Overview/607593) |
| non_hdl_cholesterol | Non-HDL cholesterol is total cholesterol minus HDL cholesterol. It captures cholesterol carried by several artery-plaque-forming particles and helps assess cardiovascular risk, including when triglycerides are elevated. | [Reference](https://medlineplus.gov/lab-tests/cholesterol-levels/) |
| cholesterol_hdl_ratio | The total cholesterol-to-HDL ratio compares total cholesterol with HDL cholesterol. It is a summary used in cardiovascular risk assessment; the individual LDL, HDL and triglyceride values still provide important information. | [Reference](https://medlineplus.gov/lab-tests/cholesterol-levels/) |
| ldl_particle_number | LDL particle number estimates the concentration of LDL particles rather than the amount of cholesterol they carry. It can help characterize cardiovascular risk when particle concentration and LDL cholesterol differ. | [Reference](https://www.questdiagnostics.com/healthcare-professionals/clinical-education-center/faq/faq134) |
| ldl_small | Small LDL measures the concentration of smaller LDL particles. It describes part of the lipoprotein pattern associated with metabolic and cardiovascular risk; it is interpreted alongside overall LDL particle burden and standard lipids. | [Reference](https://www.questdiagnostics.com/healthcare-professionals/clinical-education-center/faq/faq134) |
| ldl_medium | Medium LDL measures the concentration of medium-sized LDL particles. It adds detail about lipoprotein distribution, but its meaning depends on the overall lipid profile and the laboratory method. | [Reference](https://www.questdiagnostics.com/healthcare-professionals/clinical-education-center/faq/faq134) |
| hdl_large | Large HDL measures the concentration of larger HDL particles. It describes HDL particle distribution rather than directly measuring cholesterol removal or protection from heart disease. | [Reference](https://www.questdiagnostics.com/healthcare-professionals/clinical-education-center/faq/faq134) |
| ldl_peak_size | LDL peak size describes the predominant diameter of LDL particles. It helps characterize the particle-size pattern and is interpreted with LDL concentration, triglycerides and other metabolic risk information. | [Reference](https://www.questdiagnostics.com/healthcare-professionals/clinical-education-center/faq/faq134) |
| ldl_pattern | LDL pattern classifies the distribution of LDL particle sizes, often as predominantly larger or smaller particles. It provides additional lipid-profile context rather than a separate diagnosis. | [Reference](https://www.questdiagnostics.com/healthcare-professionals/clinical-education-center/faq/faq134) |
| lipoprotein_a | Lipoprotein(a), or Lp(a), is an LDL-like particle with an additional protein. Its level is largely inherited, and higher levels can increase the risk of cardiovascular disease and aortic valve narrowing. | [Reference](https://medlineplus.gov/lab-tests/lipoprotein-a-blood-test/) |
| homocysteine | Homocysteine is an amino acid processed using folate and vitamins B12 and B6. Testing can help investigate vitamin deficiencies or rare metabolic disorders; kidney function can also affect its level. | [Reference](https://medlineplus.gov/lab-tests/homocysteine-test/) |
| hemoglobin | Hemoglobin is the oxygen-carrying protein in red blood cells. Its concentration helps assess the blood's oxygen-carrying capacity and identify anemia or unusually high red-cell levels. | [Reference](https://medlineplus.gov/lab-tests/complete-blood-count-cbc/) |
| hematocrit | Hematocrit is the fraction of blood volume occupied by red blood cells. It is used with hemoglobin and the red-cell count to assess anemia and other blood conditions; hydration changes can affect it. | [Reference](https://medlineplus.gov/lab-tests/complete-blood-count-cbc/) |
| rbc | The red blood cell count measures how many oxygen-carrying cells are in a volume of blood. It helps investigate anemia and changes in red-cell production when interpreted with hemoglobin and cell size. | [Reference](https://medlineplus.gov/lab-tests/complete-blood-count-cbc/) |
| mcv | Mean corpuscular volume is the average size of red blood cells. Smaller or larger cells help distinguish patterns of anemia, including those related to iron, vitamin B12 or folate. | [Reference](https://medlineplus.gov/lab-tests/red-blood-cell-rbc-indices/) |
| mch | Mean corpuscular hemoglobin is the average amount of hemoglobin in each red blood cell. It helps describe the red-cell pattern in anemia and is usually interpreted with MCV and MCHC. | [Reference](https://medlineplus.gov/lab-tests/red-blood-cell-rbc-indices/) |
| mchc | Mean corpuscular hemoglobin concentration describes how concentrated hemoglobin is within red blood cells. It helps characterize anemia and some red-cell disorders rather than measuring total blood hemoglobin. | [Reference](https://medlineplus.gov/lab-tests/red-blood-cell-rbc-indices/) |
| rdw | Red cell distribution width describes how much red blood cell sizes vary. It can help distinguish anemia patterns when considered with hemoglobin and MCV; it is not a diagnosis on its own. | [Reference](https://medlineplus.gov/lab-tests/red-blood-cell-rbc-indices/) |
| platelets | Platelets are blood cell fragments that help form clots and stop bleeding. Their count helps assess bleeding or clotting problems and changes in bone marrow production. | [Reference](https://medlineplus.gov/lab-tests/complete-blood-count-cbc/) |
| mpv | Mean platelet volume describes average platelet size. It provides context about platelet production and turnover, especially alongside the platelet count; collection and analyzer methods can affect the result. | [Reference](https://medlineplus.gov/lab-tests/mpv-blood-test/) |
| wbc | The white blood cell count measures cells involved in immune defense. Changes can occur with infection, inflammation, medicines or bone marrow conditions; the differential identifies which cell types changed. | [Reference](https://medlineplus.gov/lab-tests/complete-blood-count-cbc/) |
| basophil_count | The absolute basophil count measures basophils per volume of blood. These white cells participate in allergic and inflammatory responses; interpretation uses the other blood counts and clinical context. | [Reference](https://medlineplus.gov/lab-tests/blood-differential/) |
| basophil_percent | Basophil percentage is the share of white blood cells that are basophils, which participate in allergic responses. Percentages can change when other cell types change, even if the absolute basophil count does not. | [Reference](https://medlineplus.gov/lab-tests/blood-differential/) |
| eosinophil_count | The absolute eosinophil count measures immune cells involved in allergic responses and defense against some parasites. It helps investigate allergies, medication reactions and selected inflammatory conditions. | [Reference](https://medlineplus.gov/lab-tests/blood-differential/) |
| eosinophil_percent | Eosinophil percentage describes the share of white cells involved in allergic and certain parasite responses. Read it with the absolute eosinophil count because the total white-cell composition affects percentages. | [Reference](https://medlineplus.gov/lab-tests/blood-differential/) |
| lymphocyte_count | The absolute lymphocyte count measures cells involved in antibody production and targeted immune defense. It helps assess immune and infectious conditions alongside the rest of the blood count. | [Reference](https://www.open.edu/openlearn/mod/oucontent/view.php?id=65372&section=3.1) |
| lymphocyte_percent | Lymphocyte percentage is the proportion of white cells that are lymphocytes. These cells support targeted immune defense; the percentage and absolute count can change differently. | [Reference](https://www.open.edu/openlearn/mod/oucontent/view.php?id=65372&section=3.1) |
| monocyte_count | The absolute monocyte count measures cells that help remove damaged tissue and participate in immune responses. It adds context when investigating infection or inflammation. | [Reference](https://www.open.edu/openlearn/mod/oucontent/view.php?id=65372&section=3.1) |
| monocyte_percent | Monocyte percentage shows the share of white blood cells that are monocytes. It describes immune-cell distribution and should be read with the absolute count. | [Reference](https://www.open.edu/openlearn/mod/oucontent/view.php?id=65372&section=3.1) |
| neutrophil_count | The absolute neutrophil count measures a major defense against bacterial and fungal infections. Low counts can increase infection vulnerability; increases can accompany infection, inflammation or medication effects. | [Reference](https://www.open.edu/openlearn/mod/oucontent/view.php?id=65372&section=3.1) |
| neutrophil_percent | Neutrophil percentage is the fraction of white blood cells that are neutrophils. It helps describe the immune response, while the absolute neutrophil count better reflects the number available for defense. | [Reference](https://www.open.edu/openlearn/mod/oucontent/view.php?id=65372&section=3.1) |
| immature_granulocyte_count | The absolute immature granulocyte count measures early forms of certain white blood cells circulating in blood. Their presence can reflect increased bone marrow activity during infection or inflammation and needs the rest of the blood count for context. | [Reference](https://medlineplus.gov/ency/article/003657.htm) |
| immature_granulocyte_percent | Immature granulocyte percentage is the proportion of white cells that are early granulocyte forms. It helps describe bone marrow response, and can change with infection, inflammation or other marrow stress. | [Reference](https://medlineplus.gov/ency/article/003657.htm) |
| nrbc_count | The absolute nucleated red blood cell count measures immature red cells that still contain a nucleus. In adults, these can appear with significant marrow stress or other blood conditions; newborn interpretation is different. | [Reference](https://www.sysmex.dk/academy/knowledge-centre/sysmex-parameters/nucleated-red-blood-cells-nrbc/) |
| nrbc_percent | Nucleated RBC ratio reports immature red cells relative to white blood cells. It helps identify unusual release of red-cell precursors into circulation, interpreted with age and the full blood count. | [Reference](https://www.sysmex.dk/academy/knowledge-centre/sysmex-parameters/nucleated-red-blood-cells-nrbc/) |
| creatinine | Creatinine is a waste product from muscle metabolism removed by the kidneys. It helps assess kidney filtration and calculate eGFR; muscle mass, diet and some medicines affect the result. | [Reference](https://medlineplus.gov/lab-tests/creatinine-test/) |
| bun | Blood urea nitrogen measures nitrogen in urea, a waste product formed when protein is broken down. It helps assess kidney function and fluid balance, but protein intake and other factors also influence it. | [Reference](https://medlineplus.gov/lab-tests/bun-blood-urea-nitrogen/) |
| bun_creatinine_ratio | The BUN-to-creatinine ratio compares two waste products cleared by the kidneys. It can provide context about hydration and causes of altered kidney tests, but is not interpreted without the individual values. | [Reference](https://medlineplus.gov/lab-tests/bun-blood-urea-nitrogen/) |
| egfr_creatinine_unspecified | Estimated glomerular filtration rate estimates how effectively the kidneys filter blood, usually from creatinine and personal factors. It is used to evaluate and monitor kidney function; the equation and changes over time matter. | [Reference](https://medlineplus.gov/lab-tests/glomerular-filtration-rate-gfr-test/) |
| egfr_legacy | This eGFR is a historical estimate of kidney filtration using an older equation or population adjustment. Current practice favors race-free equations; compare historical values with attention to the formula used. | [Reference](https://medlineplus.gov/lab-tests/glomerular-filtration-rate-gfr-test/) |
| albumin | Albumin is a protein made by the liver that helps keep fluid within blood vessels and carries substances through the blood. Levels help assess liver and kidney conditions, inflammation and nutritional context. | [Reference](https://medlineplus.gov/lab-tests/albumin-blood-test/) |
| total_protein | Total protein measures albumin and globulins together. These proteins support fluid balance, transport and immune defense; changes can relate to liver, kidney, immune or hydration conditions. | [Reference](https://medlineplus.gov/lab-tests/total-protein-and-albumin-globulin-a-g-ratio/) |
| globulin | Globulins are a group of blood proteins involved in immune defense and transport. The result, often calculated from total protein and albumin, helps assess immune and liver-related conditions. | [Reference](https://medlineplus.gov/lab-tests/total-protein-and-albumin-globulin-a-g-ratio/) |
| albumin_globulin_ratio | The albumin-to-globulin ratio compares two major groups of blood proteins. It adds context to liver, kidney and immune-system assessment, particularly when the individual protein values are abnormal. | [Reference](https://medlineplus.gov/lab-tests/total-protein-and-albumin-globulin-a-g-ratio/) |
| alt | Alanine aminotransferase is an enzyme found mainly in liver cells. Blood levels can rise when liver cells are injured, making it useful for monitoring liver health; it does not directly measure liver function. | [Reference](https://medlineplus.gov/lab-tests/alt-blood-test/) |
| ast | Aspartate aminotransferase is an enzyme found in the liver, muscles and other tissues. Increased levels can indicate tissue injury and are interpreted with ALT and other tests to help identify the source. | [Reference](https://medlineplus.gov/lab-tests/ast-test/) |
| alp | Alkaline phosphatase is an enzyme associated especially with bile ducts and bone. Testing helps investigate liver or bone conditions; growth, pregnancy and the source of the enzyme affect interpretation. | [Reference](https://medlineplus.gov/lab-tests/alkaline-phosphatase/) |
| ggt | Gamma-glutamyl transferase is an enzyme used to assess liver and bile-duct problems. It can help clarify whether elevated alkaline phosphatase is liver-related; alcohol and some medicines also affect it. | [Reference](https://medlineplus.gov/lab-tests/gamma-glutamyl-transferase-ggt-test/) |
| total_bilirubin | Total bilirubin measures pigment produced when red blood cells break down. The liver processes bilirubin for removal in bile; elevated levels can accompany jaundice from several different causes. | [Reference](https://medlineplus.gov/lab-tests/bilirubin-blood-test/) |
| direct_bilirubin | Direct bilirubin largely represents bilirubin processed by the liver. It helps investigate bile flow and liver conditions when interpreted with total and indirect bilirubin. | [Reference](https://medlineplus.gov/lab-tests/bilirubin-blood-test/) |
| indirect_bilirubin | Indirect bilirubin is bilirubin before liver conjugation, commonly calculated from total minus direct bilirubin. It helps assess red-cell breakdown and differences in bilirubin processing. | [Reference](https://medlineplus.gov/lab-tests/bilirubin-blood-test/) |
| sodium | Sodium is an electrolyte that helps regulate body water and supports nerve and muscle function. Its blood concentration reflects the balance of sodium and water, rather than simply dietary salt intake. | [Reference](https://medlineplus.gov/lab-tests/sodium-blood-test/) |
| potassium | Potassium is an electrolyte essential for nerve signaling, muscle contraction and heart rhythm. Blood testing helps assess kidney and medication effects; damaged blood cells during collection can falsely raise the result. | [Reference](https://medlineplus.gov/lab-tests/potassium-blood-test/) |
| chloride | Chloride is an electrolyte that works with sodium and bicarbonate to maintain fluid and acid-base balance. It is interpreted with other electrolytes when assessing hydration and metabolic conditions. | [Reference](https://medlineplus.gov/lab-tests/chloride-blood-test/) |
| carbon_dioxide | Total carbon dioxide in a chemistry panel mainly reflects bicarbonate, a buffer that helps maintain blood acidity. It helps assess acid-base balance alongside other electrolytes and kidney or breathing-related conditions. | [Reference](https://medlineplus.gov/lab-tests/carbon-dioxide-co2-in-blood/) |
| anion_gap | The anion gap is calculated from measured electrolytes to help detect unmeasured charged substances in blood. It is used when investigating acid-base disorders and depends partly on albumin and the laboratory formula. | [Reference](https://medlineplus.gov/lab-tests/anion-gap-blood-test/) |
| calcium | Calcium supports bones, teeth, muscle contraction, nerve signaling and blood clotting. Total blood calcium is influenced by albumin; calcium results help assess parathyroid, kidney and bone-related conditions. | [Reference](https://medlineplus.gov/lab-tests/calcium-blood-test/) |
| magnesium | Magnesium supports nerve and muscle activity, heart rhythm and many enzyme reactions. Blood testing helps investigate electrolyte disturbances and kidney or medication effects; serum levels do not fully reflect body stores. | [Reference](https://medlineplus.gov/lab-tests/magnesium-blood-test/) |
| rbc_magnesium | RBC magnesium measures magnesium within red blood cells rather than serum. Magnesium supports nerves, muscles and enzymes; this specialized result depends on the assay and is not interchangeable with a serum magnesium value. | [Reference](https://mlabs.umich.edu/tests/magnesium-rbc) |
| phosphorus | Phosphorus is measured in blood mainly as phosphate. It supports bone structure and cellular energy processes; levels help assess kidney function and calcium-parathyroid balance. | [Reference](https://medlineplus.gov/lab-tests/phosphate-in-blood/) |
| uric_acid | Uric acid is formed when purines are broken down and is cleared mainly by the kidneys. High levels can contribute to gout or some kidney stones, although an elevated result does not always cause symptoms. | [Reference](https://medlineplus.gov/lab-tests/uric-acid-test/) |
| amylase | Amylase is an enzyme that breaks down starch, produced mainly by the pancreas and salivary glands. Blood testing can help investigate pancreatic problems, but other conditions can also raise it. | [Reference](https://medlineplus.gov/lab-tests/amylase-test/) |
| lipase | Lipase is an enzyme that helps digest fats. Blood lipase is commonly used when evaluating pancreatic inflammation; the result is interpreted with symptoms and other findings. | [Reference](https://medlineplus.gov/lab-tests/lipase-tests/) |
| crp | C-reactive protein is made by the liver in response to inflammation. It can help detect or monitor inflammatory activity, but does not identify its location or cause. | [Reference](https://medlineplus.gov/lab-tests/c-reactive-protein-crp-test/) |
| hs_crp | High-sensitivity CRP measures low concentrations of the inflammation protein CRP. It can contribute to cardiovascular risk assessment when you are otherwise well; infection or injury can temporarily raise it. | [Reference](https://medlineplus.gov/lab-tests/c-reactive-protein-crp-test/) |
| ferritin | Ferritin is a protein that stores iron. Blood ferritin helps assess iron reserves, but it can also rise during inflammation or liver illness, so a higher value does not always mean excess iron. | [Reference](https://medlineplus.gov/lab-tests/ferritin-blood-test/) |
| iron | Serum iron measures iron circulating mainly bound to transferrin. Iron is needed for hemoglobin and oxygen transport; the result is interpreted with ferritin and binding capacity because it varies with timing and intake. | [Reference](https://medlineplus.gov/lab-tests/iron-tests/) |
| iron_binding_capacity | Iron binding capacity estimates how much iron the blood's transport proteins can bind. It is used with serum iron and ferritin to distinguish patterns of iron deficiency or excess. | [Reference](https://medlineplus.gov/lab-tests/iron-tests/) |
| iron_saturation | Iron saturation is the percentage of transferrin's iron-binding capacity occupied by iron. It helps assess whether too little or too much iron is available for tissues and red-cell production. | [Reference](https://medlineplus.gov/lab-tests/iron-tests/) |
| vitamin_d_unspecified | Vitamin D supports calcium absorption and bone and muscle health. Blood testing helps assess vitamin D status; confirm which form was measured because different vitamin D tests answer different questions. | [Reference](https://medlineplus.gov/lab-tests/vitamin-d-test/) |
| vitamin_d_25_total | Total 25-hydroxyvitamin D measures the main circulating storage form from vitamin D2 and D3. It is commonly used to assess vitamin D status in relation to calcium balance and bone health. | [Reference](https://medlineplus.gov/lab-tests/vitamin-d-test/) |
| vitamin_d_25_d3 | 25-hydroxyvitamin D3 measures the D3-derived part of circulating vitamin D. It is one component of total 25-hydroxyvitamin D and is interpreted with D2 or the total when assessing vitamin D status. | [Reference](https://medlineplus.gov/lab-tests/vitamin-d-test/) |
| zinc | Zinc is a trace mineral involved in immune function, wound healing and many enzymes. Blood testing can help assess deficiency or excess, though inflammation and collection conditions can influence levels. | [Reference](https://ods.od.nih.gov/factsheets/Zinc-HealthProfessional/) |
| lead | Blood lead testing measures exposure to a toxic metal that can harm the nervous system, blood formation and kidneys. The result is used to assess exposure and guide follow-up rather than nutritional status. | [Reference](https://medlineplus.gov/ency/article/003360.htm) |
| mercury | Mercury testing helps assess exposure to a metal that can affect the nervous system and kidneys. Blood and urine tests reflect different forms and timing of exposure, so specimen type matters. | [Reference](https://medlineplus.gov/lab-tests/heavy-metal-blood-test/) |
| mma | Methylmalonic acid is a substance that can build up when vitamin B12 is insufficient. Testing helps investigate B12 deficiency and related nerve or blood problems; reduced kidney function can also increase it. | [Reference](https://medlineplus.gov/lab-tests/methylmalonic-acid-mma-test/) |
| insulin | Insulin is a pancreatic hormone that helps glucose enter cells and regulates energy storage. Blood insulin testing helps investigate glucose regulation, especially alongside glucose and the timing of food intake. | [Reference](https://medlineplus.gov/lab-tests/insulin-in-blood-test/) |
| leptin | Leptin is a hormone produced mainly by fat tissue that signals energy stores to the brain and helps regulate appetite. Levels relate to body fat and leptin responsiveness; higher leptin does not necessarily mean stronger appetite control. | [Reference](https://www.endocrine.org/patient-engagement/endocrine-library/hormones-and-endocrine-function/endocrine-related-organs-and-hormones) |
| cortisol | Cortisol is an adrenal hormone involved in stress responses, blood pressure and metabolism. Testing helps investigate adrenal or pituitary disorders; levels normally vary by time of day and are affected by stress and medicines. | [Reference](https://medlineplus.gov/lab-tests/cortisol-test/) |
| dhea_sulfate | DHEA sulfate is an androgen precursor made mainly by the adrenal glands. It helps evaluate adrenal androgen production and some hormone-related symptoms; typical levels vary with age and sex. | [Reference](https://medlineplus.gov/lab-tests/dhea-sulfate-test/) |
| estradiol | Estradiol is an estrogen involved in reproductive function, bone maintenance and other tissues. Testing helps evaluate ovarian function and hormone treatment; age, cycle phase and medicines strongly affect the result. | [Reference](https://medlineplus.gov/lab-tests/estrogen-levels-test/) |
| fsh | Follicle-stimulating hormone comes from the pituitary and supports egg development or sperm production. It is used to investigate reproductive function and menopause together with other hormones. | [Reference](https://medlineplus.gov/lab-tests/follicle-stimulating-hormone-fsh-levels-test/) |
| lh | Luteinizing hormone comes from the pituitary and helps trigger ovulation or stimulate testosterone production. It helps assess reproductive hormone function, with timing and other hormone results providing essential context. | [Reference](https://medlineplus.gov/lab-tests/luteinizing-hormone-lh-levels-test/) |
| prolactin | Prolactin is a pituitary hormone best known for stimulating milk production. Persistently high levels can affect reproductive function; pregnancy, medicines and temporary stress can influence the result. | [Reference](https://medlineplus.gov/lab-tests/prolactin-levels/) |
| shbg | Sex hormone binding globulin is a protein that carries testosterone and estradiol in blood. Its level affects how much hormone is unbound, helping explain symptoms when total hormone levels alone are misleading. | [Reference](https://medlineplus.gov/lab-tests/shbg-blood-test/) |
| testosterone_free | Free testosterone measures the small fraction not bound to blood proteins. It helps assess available testosterone in relation to reproductive function, muscle and bone health, especially when SHBG levels are altered. | [Reference](https://medlineplus.gov/lab-tests/testosterone-levels-test/) |
| testosterone_total | Total testosterone includes both protein-bound and unbound testosterone. It helps assess androgen production and related reproductive, muscle and bone functions; levels vary with time of day and health conditions. | [Reference](https://medlineplus.gov/lab-tests/testosterone-levels-test/) |
| psa_total | Total PSA measures a protein made by prostate tissue. It is used in prostate assessment and monitoring; benign enlargement, inflammation and other factors can raise PSA, so the value does not establish cancer. | [Reference](https://medlineplus.gov/lab-tests/prostate-specific-antigen-psa-test/) |
| psa_free | Free PSA measures the portion of prostate-specific antigen not bound to other blood proteins. It can add context to a total PSA result when evaluating the need for further prostate assessment. | [Reference](https://pathlabs.ufl.edu/tests/test-directory-p/prostate-specific-antigen-psa-free/) |
| psa_free_percent | Percent free PSA is free PSA divided by total PSA. In selected clinical situations, it helps distinguish patterns associated with benign prostate conditions from those needing further investigation. | [Reference](https://pathlabs.ufl.edu/tests/test-directory-p/prostate-specific-antigen-psa-free/) |
| tsh | Thyroid-stimulating hormone is a pituitary signal that tells the thyroid to make hormones. It is commonly used with free T4 to assess thyroid function, which influences metabolism, heart rate and temperature regulation. | [Reference](https://medlineplus.gov/lab-tests/tsh-thyroid-stimulating-hormone-test/) |
| free_t4 | Free T4 measures unbound thyroxine, a thyroid hormone that helps regulate metabolism and is converted into T3 in tissues. It is interpreted with TSH when assessing thyroid function. | [Reference](https://medlineplus.gov/lab-tests/thyroxine-t4-test/) |
| free_t3 | Free T3 measures unbound triiodothyronine, an active thyroid hormone that affects metabolism and many organs. It is especially useful in some evaluations of an overactive thyroid and is interpreted with TSH and T4. | [Reference](https://medlineplus.gov/lab-tests/triiodothyronine-t3-tests/) |
| thyroglobulin | Thyroglobulin is a protein produced by thyroid tissue. It is often used to monitor certain thyroid cancers after treatment; the amount of remaining thyroid tissue and thyroglobulin antibodies strongly affect interpretation. | [Reference](https://medlineplus.gov/lab-tests/thyroglobulin/) |
| thyroglobulin_antibodies | Thyroglobulin antibodies are immune proteins directed against a thyroid protein. They can support evaluation of autoimmune thyroid disease and may interfere with thyroglobulin measurements. | [Reference](https://medlineplus.gov/lab-tests/thyroid-antibody-tests/) |
| tpo_antibodies | Thyroid peroxidase antibodies target an enzyme involved in making thyroid hormone. They help identify autoimmune thyroid disease, but thyroid hormone tests are needed to assess current thyroid function. | [Reference](https://medlineplus.gov/lab-tests/thyroid-antibody-tests/) |
| pth | Parathyroid hormone regulates blood calcium through its effects on bones, kidneys and vitamin D activation. Intact PTH testing helps investigate abnormal calcium levels and parathyroid or kidney conditions. | [Reference](https://medlineplus.gov/lab-tests/parathyroid-hormone-pth-test/) |
| iga | Immunoglobulin A is an antibody important in mucosal defenses, including the digestive and respiratory tracts. Blood levels help assess immune conditions and determine whether IgA-based celiac tests can be interpreted reliably. | [Reference](https://medlineplus.gov/lab-tests/immunoglobulins-blood-test/) |
| rheumatoid_factor | Rheumatoid factor is an antibody that can react with other antibodies. It helps investigate rheumatoid arthritis and some immune conditions, but can also occur in infections or people without arthritis. | [Reference](https://medlineplus.gov/lab-tests/rheumatoid-factor-rf-test/) |
| ana_screen | An ANA screen looks for antibodies that react with components of cell nuclei. It helps investigate certain autoimmune diseases; a positive screen can also occur without disease and requires clinical context. | [Reference](https://medlineplus.gov/lab-tests/autoantibody-testing/) |
| gliadin_iga | Deamidated gliadin IgA detects an immune response to modified gluten proteins. It can support celiac disease evaluation, depending on age, IgA status and other antibody results. | [Reference](https://www.niddk.nih.gov/health-information/professionals/clinical-tools-patient-management/digestive-diseases/celiac-disease-health-care-professionals) |
| gliadin_igg | Deamidated gliadin IgG detects a different antibody class reacting to modified gluten proteins. It may be useful in selected celiac evaluations, including when IgA is deficient. | [Reference](https://www.niddk.nih.gov/health-information/professionals/clinical-tools-patient-management/digestive-diseases/celiac-disease-health-care-professionals) |
| ttg_iga | Tissue transglutaminase IgA is a common first-line antibody test for celiac disease, which can damage the small intestine. Total IgA and ongoing gluten intake affect how the result is interpreted. | [Reference](https://www.niddk.nih.gov/health-information/professionals/clinical-tools-patient-management/digestive-diseases/celiac-disease-health-care-professionals) |
| ttg_igg | Tissue transglutaminase IgG is an antibody test used in selected celiac disease evaluations, particularly with IgA deficiency. It is not interchangeable with the more commonly used tTG-IgA test. | [Reference](https://www.niddk.nih.gov/health-information/professionals/clinical-tools-patient-management/digestive-diseases/celiac-disease-health-care-professionals) |
| urine_albumin | Urine albumin measures leakage of a blood protein into urine, which can be an early sign of kidney damage. A concentration without urine creatinine is affected by how dilute the urine is. | [Reference](https://medlineplus.gov/lab-tests/microalbumin-creatinine-ratio/) |
| abo_group | ABO blood group identifies whether red blood cells carry A or B antigens. It is important for choosing compatible blood for transfusion and is an inherited blood type, not a health score. | [Reference](https://medlineplus.gov/ency/article/003345.htm) |
| rh_factor | Rh typing usually identifies whether the RhD antigen is present on red blood cells. It matters for transfusion compatibility and pregnancy care when maternal and fetal blood types differ. | [Reference](https://medlineplus.gov/ency/article/003345.htm) |
| abo_rh | ABO/Rh combines the A, B, AB or O blood group with RhD positive or negative status. It is used to select compatible blood and support pregnancy-related care. | [Reference](https://medlineplus.gov/ency/article/003345.htm) |
| ab_screen | An antibody screen checks for unexpected antibodies against red blood cells. It helps prevent transfusion reactions and assess antibodies that may matter during pregnancy; it is different from determining ABO type. | [Reference](https://medlineplus.gov/lab-tests/red-blood-cell-antibody-screen/) |
| urine_appearance | Urine appearance describes clarity or cloudiness. Cells, crystals, mucus or bacteria can change appearance; it is interpreted with the rest of urinalysis rather than used alone to identify disease. | [Reference](https://medlineplus.gov/urinalysis.html) |
| urine_color | Urine color reflects pigments and urine concentration. Hydration, foods, medicines and blood can affect it, so color is interpreted with symptoms and the other urine findings. | [Reference](https://medlineplus.gov/urinalysis.html) |
| urine_bacteria | Urine bacteria records bacteria seen in a urine sample. They may reflect infection or contamination during collection; symptoms, white cells and sometimes a culture help distinguish these possibilities. | [Reference](https://medlineplus.gov/urinalysis.html) |
| urine_bilirubin | Urine bilirubin detects bilirubin that has been processed by the liver. Its presence can help investigate liver or bile-duct conditions and is different from blood bilirubin testing. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK557685/) |
| urine_glucose | Urine glucose detects sugar passing into urine. It can occur when blood glucose is high or because of kidney handling and certain medicines; it does not replace a blood glucose measurement. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK557685/) |
| urine_hyaline_casts | Hyaline casts are protein structures formed inside kidney tubules. Small amounts can occur with concentrated urine or exercise; quantity and other urine findings determine their significance. | [Reference](https://medlineplus.gov/ency/article/003586.htm) |
| urine_ketones | Urine ketones reflect breakdown of fat for energy. They can appear with fasting or carbohydrate restriction, but in diabetes they may also signal inadequate insulin and need prompt clinical interpretation. | [Reference](https://medlineplus.gov/lab-tests/ketones-in-urine/) |
| urine_leukocyte_esterase | Leukocyte esterase detects an enzyme associated with white blood cells in urine. It helps look for urinary inflammation or infection, though sample contamination can also affect the result. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK557685/) |
| urine_nitrite | Urine nitrite can indicate bacteria that convert nitrate to nitrite. A positive result supports evaluation for a urinary infection, but a negative result does not exclude one. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK557685/) |
| urine_occult_blood | Urine occult blood testing detects heme from red blood cells, hemoglobin or muscle pigment. It helps investigate urinary tract bleeding or other causes and is interpreted with microscopy and collection conditions. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK557685/) |
| urine_protein | Urine protein testing checks for protein passing into urine. Persistent protein can indicate kidney damage, while temporary changes may occur with exercise, fever or concentrated urine. | [Reference](https://medlineplus.gov/lab-tests/protein-in-urine/) |
| arachidonic_epa_ratio | The arachidonic acid-to-EPA ratio compares an omega-6 fatty acid with an omega-3 fatty acid. It describes fatty-acid composition, not a direct measurement of inflammation or a stand-alone treatment target. | [Reference](https://testdirectory.questdiagnostics.com/test/test-guides/TS_OmegaCheck/omegacheck?p=td) |
| omega_6_3_ratio | The omega-6-to-omega-3 ratio compares two families of fatty acids in the tested sample. It reflects relative composition; the individual fatty acids and test method are more informative than a ratio alone. | [Reference](https://testdirectory.questdiagnostics.com/test/test-guides/TS_OmegaCheck/omegacheck?p=td) |
| omega3_total | Total omega-3 measures the combined proportion of selected omega-3 fatty acids in the sample. It reflects fatty-acid composition influenced by intake and metabolism; different specimen types and assays are not interchangeable. | [Reference](https://ods.od.nih.gov/factsheets/Omega3FattyAcids-HealthProfessional/) |
| omega3_dha | DHA is an omega-3 fatty acid important in cell membranes, especially in the brain and retina. Its measured proportion describes fatty-acid status rather than directly measuring brain or eye function. | [Reference](https://ods.od.nih.gov/factsheets/Omega3FattyAcids-HealthProfessional/) |
| omega3_dpa | DPA is an omega-3 fatty acid related metabolically to EPA and DHA. This result describes its proportion in the tested sample; clinical targets for DPA alone are not established. | [Reference](https://testdirectory.questdiagnostics.com/test/test-guides/TS_OmegaCheck/omegacheck?p=td) |
| omega3_epa | EPA is an omega-3 fatty acid involved in cell membranes and signaling pathways. Its measured proportion helps describe omega-3 status and varies with dietary intake and metabolism. | [Reference](https://ods.od.nih.gov/factsheets/Omega3FattyAcids-HealthProfessional/) |
| omega6_total | Total omega-6 measures the combined proportion of selected omega-6 fatty acids. These fats contribute to cell membranes and signaling; the result is not itself a direct measure of harmful inflammation. | [Reference](https://testdirectory.questdiagnostics.com/test/test-guides/TS_OmegaCheck/omegacheck?p=td) |
| omega6_arachidonic | Arachidonic acid is an omega-6 fatty acid used in cell membranes and signaling molecules. The measured proportion describes fatty-acid composition and should not be equated with an inflammation level. | [Reference](https://testdirectory.questdiagnostics.com/test/test-guides/TS_OmegaCheck/omegacheck?p=td) |
| omega6_linoleic | Linoleic acid is an essential omega-6 fatty acid obtained from food. It contributes to cell membranes and is a precursor for other fatty acids; testing describes its proportion in the sample. | [Reference](https://testdirectory.questdiagnostics.com/test/test-guides/TS_OmegaCheck/omegacheck?p=td) |
| aptt | Activated partial thromboplastin time measures how long plasma takes to clot through selected clotting pathways. It helps investigate bleeding disorders and monitor some anticoagulants, especially unfractionated heparin. | [Reference](https://medlineplus.gov/lab-tests/partial-thromboplastin-time-ptt-test/) |
| pt | Prothrombin time measures clot formation through specific clotting pathways. It helps evaluate bleeding, liver-related clotting changes and warfarin treatment; results depend on the assay. | [Reference](https://medlineplus.gov/lab-tests/prothrombin-time-test-and-inr-ptinr/) |
| inr | The international normalized ratio standardizes prothrombin time across laboratories. It is especially used to monitor warfarin; the appropriate target depends on the reason for anticoagulation. | [Reference](https://medlineplus.gov/lab-tests/prothrombin-time-test-and-inr-ptinr/) |
| atrial_rate | Atrial rate is the rate of electrical activation in the heart's upper chambers on an ECG. Comparing it with ventricular rate helps interpret rhythm and conduction between the chambers. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK354/) |
| ventricular_rate | Ventricular rate is the rate of electrical activation in the heart's lower chambers on an ECG. It describes the recorded rhythm and is interpreted together with atrial activity and the ECG waveform. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK354/) |
| pr_interval | The PR interval is the time from the start of atrial activation to the start of ventricular activation. It helps assess electrical conduction from the upper to lower heart chambers. | [Reference](https://www.merckmanuals.com/professional/cardiovascular-disorders/cardiovascular-tests-and-procedures/electrocardiography) |
| qrs_duration | QRS duration measures how long ventricular electrical activation takes. It helps assess conduction through the lower chambers; a prolonged duration has several possible causes that require the full ECG for interpretation. | [Reference](https://www.merckmanuals.com/professional/cardiovascular-disorders/cardiovascular-tests-and-procedures/electrocardiography) |
| qt_interval | The QT interval spans ventricular activation and electrical recovery. It varies with heart rate and is relevant to medication effects, electrolyte changes and some rhythm disorders. | [Reference](https://www.merckmanuals.com/professional/cardiovascular-disorders/cardiovascular-tests-and-procedures/electrocardiography) |
| qtc | QTc adjusts the QT interval for heart rate using a formula. It helps assess ventricular electrical recovery and certain arrhythmia risks; different correction formulas can produce different values. | [Reference](https://www.merckmanuals.com/professional/cardiovascular-disorders/cardiovascular-tests-and-procedures/electrocardiography) |
| p_axis | The P-wave axis describes the overall direction of electrical activation through the atria. It adds information about atrial rhythm and conduction when interpreted with the full ECG. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK354/) |
| r_axis | The QRS or R axis describes the overall direction of ventricular electrical activation. It can help assess conduction patterns and heart orientation, but axis alone does not establish a diagnosis. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK354/) |
| t_axis | The T-wave axis describes the overall direction of ventricular electrical recovery. It is interpreted alongside the QRS axis and ECG waveforms when assessing repolarization patterns. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK354/) |
| sars_cov2_naat | A SARS-CoV-2 nucleic acid amplification test looks for genetic material from the virus that causes COVID-19. It helps identify infection; sampling timing and persistent RNA after infection can affect interpretation. | [Reference](https://www.cdc.gov/covid/testing/index.html) |
| diagnosis | This is the interpretation recorded with the source examination, rather than a numeric measurement. For an ECG, it summarizes rhythm or waveform findings and is read with the original tracing and clinical assessment. | [Reference](https://www.merckmanuals.com/professional/cardiovascular-disorders/cardiovascular-tests-and-procedures/electrocardiography) |
| specimen_expiration | Specimen expiration records when a collected sample is no longer valid for the laboratory's intended testing or compatibility work. It is a workflow date, not a measurement of body function. | [Reference](https://www.transfusionguidelines.org/red-book/chapter-13-patient-testing-red-cell-immunohaematology/13-3-pre-transfusion-testing) |
| status_information | Status information describes the processing or reporting state supplied by the laboratory or blood bank. It helps interpret the record's workflow and is not a physiologic measurement. | [Reference](https://www.transfusionguidelines.org/red-book/chapter-13-patient-testing-red-cell-immunohaematology/13-3-pre-transfusion-testing) |
| blood_bank_comment | A blood bank comment provides context for blood typing, antibody testing or transfusion compatibility. It is a laboratory note rather than a numerical health metric. | [Reference](https://www.transfusionguidelines.org/red-book/chapter-13-patient-testing-red-cell-immunohaematology/13-3-pre-transfusion-testing) |

| p_wave | The P wave represents electrical activation of the heart's upper chambers, the atria. Its shape and timing help assess where the rhythm starts and how atrial activation spreads; it is interpreted as part of the full ECG. | [Reference](https://www.ncbi.nlm.nih.gov/books/NBK354/) |
| qrs_complex | The QRS complex represents electrical activation of the heart's lower chambers, the ventricles. Its shape, duration and direction help assess ventricular conduction and rhythm; the original ECG identifies which feature was reported. | [Reference](https://www.merckmanuals.com/professional/cardiovascular-disorders/cardiovascular-tests-and-procedures/electrocardiography) |

## Observed-name coverage

| Observed name | Explanation key |
| --- | --- |
| 25-Hydroxyvitamin D3+25-Hydroxyvitamin D2 [Mass/volume] in Serum or Plasma | vitamin_d_25_total |
| 25-hydroxyvitamin D3 [Mass/volume] in Serum or Plasma | vitamin_d_25_d3 |
| AB Screen | ab_screen |
| ABO Group | abo_group |
| ABO and Rh group [Type] in Blood | abo_rh |
| ABORh | abo_rh |
| Absolute Basophil Count | basophil_count |
| Absolute Eosinophil Count | eosinophil_count |
| Absolute Lymphocyte Count | lymphocyte_count |
| Absolute Monocyte Count | monocyte_count |
| Absolute NRBC | nrbc_count |
| Absolute Neutrophil Count | neutrophil_count |
| Activated Partial Thromboplastin Time | aptt |
| Alanine Aminotransferase | alt |
| Alanine Aminotransferase (ALT) | alt |
| Alanine aminotransferase [Enzymatic activity/volume] in Serum or Plasma | alt |
| Albumin | albumin |
| Albumin / Globulin Ratio | albumin_globulin_ratio |
| Albumin Level | albumin |
| Albumin [Mass/volume] in Serum or Plasma | albumin |
| Albumin, Random Urine without Creatinine | urine_albumin |
| Alkaline Phosphatase | alp |
| Alkaline Phosphatase (ALP) | alp |
| Alkaline phosphatase | alp |
| Alkaline phosphatase [Enzymatic activity/volume] in Serum or Plasma | alp |
| Amylase | amylase |
| Anion Gap | anion_gap |
| Anion gap in Serum or Plasma by calculation | anion_gap |
| Antinuclear Antibodies (ANA) Screen | ana_screen |
| Apolipoprotein B (ApoB) | apob |
| Appearance - Urine | urine_appearance |
| Arachidonic Acid/EPA Ratio | arachidonic_epa_ratio |
| Aspartate Aminotransferase | ast |
| Aspartate Aminotransferase (AST) | ast |
| Aspartate aminotransferase [Enzymatic activity/volume] in Serum or Plasma | ast |
| Atrial Rate | atrial_rate |
| BUN/Creatinine Ratio | bun_creatinine_ratio |
| BUN/creatinine ratio | bun_creatinine_ratio |
| Bacteria - Urine | urine_bacteria |
| Basophil Automated | basophil_percent |
| Basophils | basophil_count |
| Basophils % | basophil_percent |
| Basophils [#/volume] in Blood by Automated count | basophil_count |
| Basophils/Leukocytes in Blood by Automated count | basophil_percent |
| Bilirubin - Urine | urine_bilirubin |
| Bilirubin Direct | direct_bilirubin |
| Bilirubin Indirect | indirect_bilirubin |
| Bilirubin Total | total_bilirubin |
| Bilirubin.direct [Mass/volume] in Serum or Plasma | direct_bilirubin |
| Bilirubin.indirect [Mass/volume] in Serum or Plasma | indirect_bilirubin |
| Bilirubin.total [Mass/volume] in Serum or Plasma | total_bilirubin |
| Blood Urea Nitrogen | bun |
| Blood Urea Nitrogen (BUN) | bun |
| Blood bank comment | blood_bank_comment |
| Blood group antibody screen [Presence] in Serum or Plasma | ab_screen |
| Calcium | calcium |
| Calcium Level Total | calcium |
| Calcium Level with PTH | calcium |
| Calcium [Mass/volume] in Serum or Plasma | calcium |
| Carbon Dioxide | carbon_dioxide |
| Carbon dioxide, total [Moles/volume] in Serum or Plasma | carbon_dioxide |
| Celiac - Deamidated Gliadin (IgA) | gliadin_iga |
| Celiac - Deamidated Gliadin (IgG) | gliadin_igg |
| Celiac - Tissue Transglutaminase (IgA) | ttg_iga |
| Celiac - Tissue Transglutaminase (IgG) | ttg_igg |
| Chloride | chloride |
| Chloride Level | chloride |
| Chloride [Moles/volume] in Serum or Plasma | chloride |
| Color - Urine | urine_color |
| Cortisol | cortisol |
| Creatinine | creatinine |
| Creatinine [Mass/volume] in Serum or Plasma | creatinine |
| Creatinine-Based Estimated Glomerular Filtration Rate (eGFR) | egfr_creatinine_unspecified |
| DHEA Sulfate | dhea_sulfate |
| DIAGNOSIS | diagnosis |
| EGFR African American | egfr_legacy |
| Eosinophil Automated | eosinophil_percent |
| Eosinophils | eosinophil_count |
| Eosinophils % | eosinophil_percent |
| Eosinophils [#/volume] in Blood by Automated count | eosinophil_count |
| Eosinophils/Leukocytes in Blood by Automated count | eosinophil_percent |
| Erythrocyte [DistWidth] in Blood by Automated count | rdw |
| Erythrocytes [#/volume] in Blood by Automated count | rbc |
| Estradiol (E2) | estradiol |
| Ferritin | ferritin |
| Follicle Stimulating Hormone (FSH) | fsh |
| GFR estimate | egfr_creatinine_unspecified |
| Gamma-glutamyl Transferase (GGT) | ggt |
| Globulin | globulin |
| Globulin [Mass/volume] in Serum | globulin |
| Glomerular filtration rate [Volume Rate/Area] in Serum, Plasma or Blood by Creatinine-based formula (CKD-EPI)/1.73 sq M among black population | egfr_legacy |
| Glomerular filtration rate [Volume Rate/Area] in Serum, Plasma or Blood by Creatinine-based formula (CKD-EPI)/1.73 sq M among non black population | egfr_legacy |
| Glomerular filtration rate [Volume Rate/Area] in Serum, Plasma or Blood by Creatinine-based formula (MDRD)/1.73 sq M | egfr_creatinine_unspecified |
| Glomerular filtration rate [Volume Rate/Area] in Serum, Plasma or Blood by based on 1.73 sq M | egfr_creatinine_unspecified |
| Glomerular filtration rate/1.73 sq M.predicted among blacks [Volume Rate/Area] in Serum, Plasma or Blood by Creatinine-based formula (CKD-EPI) | egfr_legacy |
| Glomerular filtration rate/1.73 sq M.predicted among non-blacks [Volume Rate/Area] in Serum, Plasma or Blood by Creatinine-based formula (CKD-EPI) | egfr_legacy |
| Glucose | glucose |
| Glucose - Urine | urine_glucose |
| Glucose Level | glucose |
| Glucose [Mass/volume] in Serum or Plasma | glucose |
| HDL Large | hdl_large |
| HDL-Cholesterol | hdl_cholesterol |
| Hematocrit | hematocrit |
| Hematocrit [Volume Fraction] of Blood by Automated count | hematocrit |
| Hemoglobin | hemoglobin |
| Hemoglobin A1c (HbA1c) | hba1c |
| Hemoglobin [Mass/volume] in Blood | hemoglobin |
| High-Sensitivity C-Reactive Protein (hs-CRP) | hs_crp |
| Higher GFR estimate (approximate) | egfr_legacy |
| Homocysteine | homocysteine |
| Hyaline Casts - Urine | urine_hyaline_casts |
| INR in Platelet poor plasma by Coagulation assay | inr |
| Imm Gran % | immature_granulocyte_percent |
| Imm Gran Abs | immature_granulocyte_count |
| Immature granulocytes [#/volume] in Blood | immature_granulocyte_count |
| Immature granulocytes/Leukocytes in Blood | immature_granulocyte_percent |
| Immunoglobulin A (IgA) | iga |
| Insulin | insulin |
| International Normalization Ratio | inr |
| Iron | iron |
| Iron % Saturation | iron_saturation |
| Iron Binding Capacity | iron_binding_capacity |
| Ketones - Urine | urine_ketones |
| LDL Medium | ldl_medium |
| LDL Particle Number | ldl_particle_number |
| LDL Pattern | ldl_pattern |
| LDL Peak Size | ldl_peak_size |
| LDL Small | ldl_small |
| LDL-Cholesterol | ldl_cholesterol |
| Lead | lead |
| Leptin | leptin |
| Leukocyte Esterase - Urine | urine_leukocyte_esterase |
| Leukocytes [#/volume] in Blood by Automated count | wbc |
| Lipase | lipase |
| Lipase Level | lipase |
| Lipase [Enzymatic activity/volume] in Serum or Plasma | lipase |
| Lipoprotein (a) | lipoprotein_a |
| Lower GFR estimate (approximate) | egfr_legacy |
| Luteinizing Hormone (LH) | lh |
| Lymphocyte Automated | lymphocyte_percent |
| Lymphocytes | lymphocyte_count |
| Lymphocytes % | lymphocyte_percent |
| Lymphocytes [#/volume] in Blood by Automated count | lymphocyte_count |
| Lymphocytes/Leukocytes in Blood by Automated count | lymphocyte_percent |
| MCH [Entitic mass] by Automated count | mch |
| MCHC [Entitic Mass/volume] in Red Blood Cells by Automated count | mchc |
| MCV [Entitic mean volume] in Red Blood Cells by Automated count | mcv |
| Magnesium | magnesium |
| Magnesium [Mass/volume] in Serum or Plasma | magnesium |
| Magnesium, RBC | rbc_magnesium |
| Mean Cell Hemoglobin | mch |
| Mean Cell Hemoglobin Concentration | mchc |
| Mean Cell Volume | mcv |
| Mean Corpuscular Hemoglobin (MCH) | mch |
| Mean Corpuscular Hemoglobin Concentration (MCHC) | mchc |
| Mean Corpuscular Volume (MCV) | mcv |
| Mean Platelet Volume | mpv |
| Mean Platelet Volume (MPV) | mpv |
| Mean corpuscular volume | mcv |
| Mercury | mercury |
| Methylmalonic Acid (MMA) | mma |
| Monocyte Automated | monocyte_percent |
| Monocytes | monocyte_count |
| Monocytes % | monocyte_percent |
| Monocytes [#/volume] in Blood by Automated count | monocyte_count |
| Monocytes/Leukocytes in Blood by Automated count | monocyte_percent |
| Neutrophil Automated | neutrophil_percent |
| Neutrophils | neutrophil_count |
| Neutrophils % | neutrophil_percent |
| Neutrophils [#/volume] in Blood by Automated count | neutrophil_count |
| Neutrophils/Leukocytes in Blood by Automated count | neutrophil_percent |
| Nitrite - Urine | urine_nitrite |
| Non-HDL Cholesterol | non_hdl_cholesterol |
| Nucleated RBC Auto | nrbc_percent |
| Nucleated erythrocytes [#/volume] in Blood by Automated count | nrbc_count |
| Nucleated erythrocytes/Leukocytes [Ratio] in Blood by Automated count | nrbc_percent |
| Occult Blood - Urine | urine_occult_blood |
| Omega 6 / 3 Ratio | omega_6_3_ratio |
| Omega-3 Total | omega3_total |
| Omega-3 Total / OmegaCheck | omega3_total |
| Omega-3: DHA | omega3_dha |
| Omega-3: DPA | omega3_dpa |
| Omega-3: EPA | omega3_epa |
| Omega-6 Total | omega6_total |
| Omega-6: Arachidonic Acid | omega6_arachidonic |
| Omega-6: Linoleic Acid | omega6_linoleic |
| P Axis | p_axis |
| P wave Atrium by EKG | p_wave |
| P wave axis | p_axis |
| P-R Interval | pr_interval |
| Parathyrin.intact [Mass/volume] in Serum or Plasma | pth |
| Parathyroid Hormone Intact | pth |
| Parathyroid Hormone,Intact | pth |
| Phosphate (as Phosphorus) | phosphorus |
| Phosphate [Mass/volume] in Serum or Plasma | phosphorus |
| Platelet | platelets |
| Platelet Count | platelets |
| Platelet [Entitic mean volume] in Blood by Automated count | mpv |
| Platelets [#/volume] in Blood by Automated count | platelets |
| Potassium | potassium |
| Potassium Level | potassium |
| Potassium [Moles/volume] in Serum or Plasma | potassium |
| Prolactin | prolactin |
| Prostate Specific Antigen (PSA) %, Free | psa_free_percent |
| Prostate Specific Antigen (PSA), Free | psa_free |
| Prostate Specific Antigen (PSA), Total | psa_total |
| Protein - Urine | urine_protein |
| Protein Total | total_protein |
| Protein [Mass/volume] in Serum or Plasma | total_protein |
| Prothrombin Time | pt |
| Prothrombin time (PT) | pt |
| Q-T Interval | qt_interval |
| Q-T interval | qt_interval |
| Q-T interval corrected | qtc |
| QRS Duration | qrs_duration |
| QRS complex Ventricles by EKG | qrs_complex |
| QRS duration | qrs_duration |
| QTc | qtc |
| R Axis | r_axis |
| R wave axis | r_axis |
| Red Blood Cell | rbc |
| Red Blood Cell (RBC) Count | rbc |
| Red Cell Distribution Width | rdw |
| Red Cell Distribution Width (RDW) | rdw |
| Red cell distribution width (CV) | rdw |
| Rhesus (Rh) Factor | rh_factor |
| Rheumatoid Factor (RF) | rheumatoid_factor |
| SARS-CoV-2 (COVID-19) RNA [Presence] in Respiratory system specimen by NAA with probe detection | sars_cov2_naat |
| SARS-CoV-2 NAAT | sars_cov2_naat |
| Sex Hormone Binding Globulin (SHBG) | shbg |
| Sodium | sodium |
| Sodium Level | sodium |
| Sodium [Moles/volume] in Serum or Plasma | sodium |
| Specimen Expiration Date | specimen_expiration |
| Specimen expiration date of Blood | specimen_expiration |
| Status Information | status_information |
| T Axis | t_axis |
| T wave axis | t_axis |
| T4, Free | free_t4 |
| T4, free | free_t4 |
| TSH | tsh |
| Testosterone, Free | testosterone_free |
| Testosterone, Total | testosterone_total |
| Thyroglobulin | thyroglobulin |
| Thyroglobulin Ab [Units/volume] in Serum | thyroglobulin_antibodies |
| Thyroglobulin Ab [Units/volume] in Serum or Plasma | thyroglobulin_antibodies |
| Thyroglobulin Antibodies | thyroglobulin_antibodies |
| Thyroglobulin Antibodies (TgAb) | thyroglobulin_antibodies |
| Thyroglobulin [Mass/volume] in Serum or Plasma | thyroglobulin |
| Thyroid Peroxidase Antibodies (TPO) | tpo_antibodies |
| Thyroid Stimulating Hormone | tsh |
| Thyroid-Stimulating Hormone (TSH) | tsh |
| Thyrotropin [Units/volume] in Serum or Plasma | tsh |
| Thyrotropin [Units/volume] in Serum or Plasma by Detection limit <= 0.005 mIU/L | tsh |
| Thyroxine (T4) Free | free_t4 |
| Thyroxine (T4) free [Mass/volume] in Serum or Plasma | free_t4 |
| Total Bilirubin | total_bilirubin |
| Total Cholesterol | total_cholesterol |
| Total Cholesterol / HDL Ratio | cholesterol_hdl_ratio |
| Total Protein | total_protein |
| Triglycerides | triglycerides |
| Triiodothyronine (T3) Free | free_t3 |
| Urea Nitrogen (BUN) | bun |
| Urea nitrogen [Mass/volume] in Serum or Plasma | bun |
| Urea nitrogen/Creatinine [Mass Ratio] in Serum or Plasma | bun_creatinine_ratio |
| Uric Acid | uric_acid |
| Ventricular Rate | ventricular_rate |
| Vitamin D | vitamin_d_unspecified |
| Vitamin D,25-OH, Total, IA | vitamin_d_25_total |
| White Blood Cell | wbc |
| White Blood Cell (WBC) Count | wbc |
| White blood cell count | wbc |
| Zinc | zinc |
| aPTT in Platelet poor plasma by Coagulation assay | aptt |
| albumin | albumin |
| calcium | calcium |
| carbon dioxide | carbon_dioxide |
| chloride | chloride |
| creatinine | creatinine |
| eGFR Non Afr American | egfr_legacy |
| eGFR african american | egfr_legacy |
| eGFR non-afr. american | egfr_legacy |
| glucose | glucose |
| parathyroid hormone, intact | pth |
| potassium | potassium |
| sodium | sodium |
| thyroglobulin | thyroglobulin |
| thyroglobulin antibodies | thyroglobulin_antibodies |
| urea nitrogen (BUN) | bun |
| vitamin D,25-oh,total,ia | vitamin_d_25_total |
