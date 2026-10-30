"""Data-driven Apple Health Browse acceptance against the synthetic fixture."""
import time
import ui_smoke as ui

ui.tap('Browse')
ui.wait_for('Sleep')

def search(text):
    fields=[n for n in ui.tree().iter('node') if n.get('class')=='android.widget.EditText']
    assert fields
    ui.tap_node(fields[0]);ui.adb('shell','input','keyevent','KEYCODE_MOVE_END')
    ui.adb('shell','input','keycombination','KEYCODE_CTRL_LEFT','KEYCODE_A')
    ui.adb('shell','input','keyevent','KEYCODE_DEL')
    ui.adb('shell','input','text',text.replace(' ','%s'));ui.adb('shell','input','keyevent','KEYCODE_BACK')

for query,title in [('body mass index','Body Mass Index (BMI)'),('systolic','Systolic Blood Pressure'),('physical effort','Physical Effort')]:
    search(query);ui.wait_for(title);ui.tap(title);ui.wait_for('RANGE')
    assert 'No recorded measurements in this period.' not in ui.labels()
    ui.tap('Y');ui.wait_for('Apple Watch')
    ui.tap('Apple Watch');ui.wait_for('No sources selected')
    ui.tap('Apple Watch');ui.wait_for('RANGE')
    print('PASS:',title,'chart, year and source toggles')
search('stand hour');ui.wait_for('Apple Stand Hour');ui.tap('Apple Stand Hour')
ui.wait_for('Historical Values');ui.wait_for('Apple Watch');ui.tap('Apple Watch');ui.wait_for('RECORDED DETAILS')
assert any('Stood' in (v or '') for v in ui.labels())
print('PASS: health event history and source detail')
