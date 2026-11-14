export function el(tag, text, className) {
  const node = document.createElement(tag);
  if (text !== null && text !== undefined) node.textContent = String(text);
  if (className) node.className = className;
  return node;
}
export function button(text, action, className) {
  const node = el('button', text, className);
  node.type = 'button';
  node.addEventListener('click', action);
  return node;
}
export function badge(status) { return el('span', (status ?? 'unknown').replaceAll('_', ' '), 'badge'); }
export function dated(seconds) {
  return seconds ? new Date(seconds * 1000).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' }) : 'Not available';
}
export function numeric(value) {
  return value === null || value === undefined ? 'No visible samples' : Number(value).toLocaleString(undefined, { maximumFractionDigits: 3 });
}
export function quantity(value) {
  if (!value?.display) return value?.reason ?? 'Not available';
  return value.display;
}
export function originalReference(payload, reference) {
  if (!payload.reference_range) return 'Not printed';
  const origin = reference.unit_origin === 'reference_explicit' ? 'explicit unit' : 'result-column unit';
  return `${payload.reference_range} · ${reference.original_unit ?? 'unit unknown'}${reference.original_unit ? ` (${origin})` : ''}`;
}
export function detail(title, value) {
  const node = el('details');
  node.append(el('summary', title), el('pre', typeof value === 'string' ? value : JSON.stringify(value, null, 2)));
  return node;
}
export function table(headings, rows) {
  const wrap = el('div', null, 'table-wrap');
  const node = el('table');
  const head = el('thead');
  const tr = el('tr');
  for (const name of headings) {
    const cell = el('th', name);
    cell.scope = 'col';
    tr.append(cell);
  }
  head.append(tr);
  const body = el('tbody');
  for (const row of rows) {
    const tr = el('tr');
    for (const value of row) {
      const td = el('td');
      td.append(value instanceof Node ? value : el('span', value ?? '—'));
      tr.append(td);
    }
    body.append(tr);
  }
  node.append(head, body);
  wrap.append(node);
  return wrap;
}
