pub fn find_element_js() -> &'static str {
    r#"
    const SKIP_TAGS = new Set(['SCRIPT','STYLE','NOSCRIPT','TEMPLATE','SVG','PATH','DEFS','CLIPPATH','LINEARGRADIENT','STOP','META','LINK','BR','WBR']);
    const INTERACTIVE_ROLES = new Set(['button','link','textbox','checkbox','radio','combobox','listbox','menuitem','menuitemcheckbox','menuitemradio','option','searchbox','slider','spinbutton','switch','tab','treeitem']);

    function getRole(el) {
        const explicit = el.getAttribute('role');
        if (explicit) return explicit.toLowerCase();
        const tag = el.tagName.toUpperCase();
        if (tag === 'H3') return 'heading';
        const roleMap = {
            'A': el.hasAttribute('href') ? 'link' : 'generic',
            'BUTTON': 'button', 'INPUT': getInputRole(el), 'SELECT': 'combobox', 'TEXTAREA': 'textbox', 'IMG': 'img',
            'H1':'heading','H2':'heading','H3':'heading','H4':'heading','H5':'heading','H6':'heading',
            'NAV':'navigation','MAIN':'main','HEADER':'banner','FOOTER':'contentinfo','ASIDE':'complementary',
            'FORM':'form','TABLE':'table','UL':'list','OL':'list','LI':'listitem',
            'DETAILS':'group','SUMMARY':'button','DIALOG':'dialog'
        };
        return roleMap[tag] || 'generic';
    }

    function getInputRole(el) {
        const type = (el.getAttribute('type') || 'text').toLowerCase();
        const map = {'text':'textbox','email':'textbox','password':'textbox','search':'searchbox','tel':'textbox','url':'textbox','number':'spinbutton','checkbox':'checkbox','radio':'radio','submit':'button','reset':'button','button':'button','range':'slider'};
        return map[type] || 'textbox';
    }

    function getAccessibleName(el) {
        const ariaLabel = el.getAttribute('aria-label') || el.getAttribute('title') || el.getAttribute('placeholder');
        if (ariaLabel) return ariaLabel.trim();
        const tag = el.tagName.toUpperCase();
        if (tag === 'IMG') return el.getAttribute('alt') || '';
        if (tag === 'A' && el.querySelector('h3')) {
            return el.querySelector('h3').textContent.trim();
        }
        const text = el.innerText || el.textContent || '';
        return text.trim().substring(0, 100);
    }

    function isVisible(el) {
        if (el.hidden || el.getAttribute('aria-hidden') === 'true') return false;
        const style = getComputedStyle(el);
        if (style.display === 'none' || style.visibility === 'hidden' || parseFloat(style.opacity) === 0) return false;
        const rect = el.getBoundingClientRect();
        // Be more lenient on width/height for inline elements
        return rect.width >= 0 && rect.height >= 0;
    }

    function __findElement(selector) {
        const refMatch = selector.match(/^\[ref=(e\d+)\]$/);
        if (refMatch) selector = '@' + refMatch[1];

        if (selector.startsWith('@e')) {
            const targetNum = parseInt(selector.slice(2));
            let counter = 0;
            const walk = (root) => {
                if (!root) return null;
                const children = root.shadowRoot ? Array.from(root.children).concat(Array.from(root.shadowRoot.children)) : Array.from(root.children);
                for (const node of children) {
                    if (SKIP_TAGS.has(node.tagName.toUpperCase())) continue;
                    if (!isVisible(node)) continue;
                    if (node.tagName.toUpperCase() === 'IFRAME') {
                        try { const found = walk(node.contentDocument.body); if (found) return found; } catch(e) {}
                    }
                    const role = getRole(node);
                    const name = getAccessibleName(node);
                    if (INTERACTIVE_ROLES.has(role) || (role !== 'generic' && name)) {
                        counter++;
                        if (counter === targetNum) return node;
                    }
                    const found = walk(node);
                    if (found) return found;
                }
                return null;
            }
            return walk(document.body || document.documentElement);
        }

        if (selector.startsWith('//') || selector.startsWith('(//')) {
            try { return document.evaluate(selector, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue; } catch(e) {}
        }

        const deepQuery = (root, sel) => {
            if (!root) return null;
            // Handle space-separated selectors as a whole first
            try {
                let el = root.querySelector(sel);
                if (el && isVisible(el)) return el;
            } catch(e) {}

            for (const node of root.querySelectorAll('*')) {
                if (node.shadowRoot) {
                    const found = deepQuery(node.shadowRoot, sel);
                    if (found) return found;
                }
            }
            return null;
        }

        // Handle :contains pseudo-class manually
        const containsMatch = selector.match(/^(.*):contains\(['"]?(.*?)['"]?\)$/);
        if (containsMatch) {
            const baseSelector = containsMatch[1] || '*';
            const textToFind = containsMatch[2].toLowerCase();
            const candidates = document.querySelectorAll(baseSelector);
            for (const node of candidates) {
                if (!isVisible(node)) continue;
                if ((node.innerText || node.textContent || '').toLowerCase().includes(textToFind)) {
                    return node;
                }
            }
            return null;
        }

        const res = deepQuery(document, selector);
        if (res) return res;

        // Smart Text Search
        const text = selector.toLowerCase().trim();
        const findByText = (root) => {
            if (!root) return null;

            // 1. Prioritize Inputs (for "Type" actions)
            const inputs = root.querySelectorAll('input, textarea');
            for (const input of inputs) {
                if (!isVisible(input)) continue;
                const p = (input.getAttribute('placeholder') || '').toLowerCase();
                const n = (input.getAttribute('name') || '').toLowerCase();
                const i = (input.id || '').toLowerCase();
                const a = (input.getAttribute('aria-label') || '').toLowerCase();

                // Exact match priority
                if (n === text || i === text) return input;
                // Partial match
                if (p.includes(text) || a.includes(text)) return input;
            }

            // 2. Search Clickables (Buttons/Links) - specific text match
            const clickables = root.querySelectorAll('a, button, [role="button"], [role="link"], label');
            for (const node of clickables) {
                if (!isVisible(node)) continue;
                const inner = (node.innerText || node.textContent || '').toLowerCase().trim();
                const aria = (node.getAttribute('aria-label') || '').toLowerCase().trim();
                const title = (node.getAttribute('title') || '').toLowerCase().trim();

                // Exact match gets highest priority
                if (inner === text || aria === text || title === text) return node;
                // Strong partial match
                if (inner.includes(text) && text.length > 3) return node;
            }

            // 3. Search Headings (return parent link if exists)
            const headings = root.querySelectorAll('h1, h2, h3, h4, h5, h6');
            for (const h of headings) {
                if (!isVisible(h)) continue;
                if (h.textContent.toLowerCase().includes(text)) {
                    return h.closest('a') || h;
                }
            }

            // 4. Fallback: Search all interactive elements looser
            const nodes = root.querySelectorAll('button, a, [role="button"], [role="link"], label');
            for (const node of nodes) {
                const inner = (node.innerText || node.textContent || '').toLowerCase();
                if (inner.includes(text) && isVisible(node)) return node;
            }
            return null;
        }
        return findByText(document.body);
    }

    function __scanPage() {
        let counter = 0;
        let lines = [];
        let elements = {};
        const walk = (root) => {
            if (!root) return;
            const children = root.shadowRoot ? Array.from(root.children).concat(Array.from(root.shadowRoot.children)) : Array.from(root.children);
            for (const node of children) {
                if (SKIP_TAGS.has(node.tagName.toUpperCase())) continue;
                if (!isVisible(node)) continue;
                if (node.tagName.toUpperCase() === 'IFRAME') {
                    try { walk(node.contentDocument.body); } catch(e) {}
                }
                const role = getRole(node);
                const name = getAccessibleName(node);
                if (INTERACTIVE_ROLES.has(role) || (role !== 'generic' && name)) {
                    counter++;
                    const ref = `e${counter}`;
                    elements[ref] = { role, name };
                    lines.push(`[ref=${ref}] ${role}: ${name}`);
                    node.setAttribute('data-openspore-ref', ref);
                }
                walk(node);
            }
        }
        walk(document.body || document.documentElement);
        return { snapshot: lines.join('\n'), elements };
    }

    function __findElementBySemantic(role, name) {
        // Normalization helper
        const norm = (s) => (s || '').toLowerCase().trim();
        const targetRole = norm(role);
        const targetName = norm(name);

        const find = (root, mode) => {
            if (!root) return null;
            const children = root.shadowRoot ? Array.from(root.children).concat(Array.from(root.shadowRoot.children)) : Array.from(root.children);
            for (const node of children) {
                if (SKIP_TAGS.has(node.tagName.toUpperCase())) continue;

                // Allow hidden elements if we are searching desperately (mode 2)
                if (mode < 2 && !isVisible(node)) continue;

                if (node.tagName.toUpperCase() === 'IFRAME') {
                    try { const res = find(node.contentDocument.body, mode); if (res) return res; } catch(e) {}
                }

                const r = norm(getRole(node));
                const n = norm(getAccessibleName(node));

                // Mode 0: Exact Match
                if (mode === 0) {
                    if (r === targetRole && n === targetName) return node;
                }
                // Mode 1: Fuzzy Name contain match (if role matches)
                else if (mode === 1) {
                     if (r === targetRole && n.includes(targetName)) return node;
                }

                const found = find(node, mode);
                if (found) return found;
            }
            return null;
        }

        const root = document.body || document.documentElement;
        // Try strict first
        let el = find(root, 0);
        if (el) return el;
        // Try fuzzy
        el = find(root, 1);
        if (el) return el;

        return null;
    }
    "#
}
