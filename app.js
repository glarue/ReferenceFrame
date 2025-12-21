/**
 * ReferenceFrame - JavaScript initialization
 *
 * This file handles:
 * - PDF export with vector SVG (jsPDF helper)
 * - URL parameter decoding for shareable designs
 * - Instant settings restoration from localStorage
 * - UI tab switching before PyScript loads
 */

/**
 * Binary URL format decoder
 * Format: ?d=<base64>
 *
 * Binary structure (28 bytes):
 *   5 × uint24: h, w, mw, fw, fd (×10000 for 4 decimal precision)
 *   6 × uint16: gt, mt, at, bt, rd, bw (×10000 for 4 decimal precision)
 *   1 × byte: flags (bit 0 = mat, bit 1 = unit_mm)
 */
function decodeFrameConfig(base64Str) {
    try {
        // Add padding if needed
        while (base64Str.length % 4 !== 0) {
            base64Str += '=';
        }
        // Convert URL-safe base64 to standard
        const standardB64 = base64Str.replace(/-/g, '+').replace(/_/g, '/');
        const binary = atob(standardB64);

        if (binary.length !== 28) {
            console.error('Invalid config length:', binary.length);
            return null;
        }

        const bytes = new Uint8Array(binary.length);
        for (let i = 0; i < binary.length; i++) {
            bytes[i] = binary.charCodeAt(i);
        }

        // Decode uint24 values (h, w, mw, fw, fd)
        const uint24Fields = [];
        for (let i = 0; i < 5; i++) {
            const offset = i * 3;
            const val = (bytes[offset] << 16) | (bytes[offset + 1] << 8) | bytes[offset + 2];
            uint24Fields.push(val / 10000);
        }

        // Decode uint16 values (gt, mt, at, bt, rd, bw)
        const uint16Fields = [];
        const uint16Start = 15;
        for (let i = 0; i < 6; i++) {
            const offset = uint16Start + i * 2;
            const val = (bytes[offset] << 8) | bytes[offset + 1];
            uint16Fields.push(val / 10000);
        }

        // Decode flags
        const flags = bytes[27];
        const includeMat = (flags & 1) === 1;
        const unitMm = ((flags >> 1) & 1) === 1;

        return {
            h: uint24Fields[0],
            w: uint24Fields[1],
            mw: uint24Fields[2],
            fw: uint24Fields[3],
            fd: uint24Fields[4],
            gt: uint16Fields[0],
            mt: uint16Fields[1],
            at: uint16Fields[2],
            bt: uint16Fields[3],
            rd: uint16Fields[4],
            bw: uint16Fields[5],
            mat: includeMat,
            unit_mm: unitMm
        };
    } catch (e) {
        console.error('Error decoding config:', e);
        return null;
    }
}

/**
 * Apply decoded config to form fields
 */
function applyConfigToForm(config, unitMm) {
    const convert = unitMm ? (v) => v * 25.4 : (v) => v;

    const mappings = {
        'artwork-height': config.h,
        'artwork-width': config.w,
        'mat-width': config.mw,
        'frame-width': config.fw,
        'frame-depth': config.fd,
        'glazing-thickness': config.gt,
        'matboard-thickness': config.mt,
        'artwork-thickness': config.at,
        'backing-thickness': config.bt,
        'rabbet-depth': config.rd,
        'blade-width': config.bw
    };

    for (const [fieldId, value] of Object.entries(mappings)) {
        const element = document.getElementById(fieldId);
        if (element) {
            // Convert to display unit and round appropriately
            const displayValue = convert(value);
            element.value = unitMm ? displayValue.toFixed(2) : displayValue.toString();
        }
    }

    // Set mat checkbox
    const matCheckbox = document.getElementById('include-mat');
    if (matCheckbox) {
        matCheckbox.checked = config.mat;
    }

    // Set unit toggle
    if (unitMm) {
        document.getElementById('unit-mm')?.classList.add('active');
        document.getElementById('unit-inches')?.classList.remove('active');
        localStorage.setItem('frame_designer_unit', 'mm');
    } else {
        document.getElementById('unit-inches')?.classList.add('active');
        document.getElementById('unit-mm')?.classList.remove('active');
        localStorage.setItem('frame_designer_unit', 'inches');
    }
}

// Flag to indicate URL params were loaded (Python can check this)
window.rfLoadedFromUrl = false;

// Cache jsPDF constructor at load time (before Pyodide might interfere)
const _jsPDF = window.jspdf.jsPDF;
console.log("Cached _jsPDF, test pdf.svg:", typeof (new _jsPDF()).svg);

/**
 * Create PDF with vector SVG diagram
 * Called from Python PyScript code
 */
window.createPdfWithVectorSvg = async function(x, y, width, height) {
    console.log("createPdfWithVectorSvg called");

    const svg = document.querySelector("#matplotlib-canvas svg");
    if (!svg) {
        console.error("No SVG found");
        return null;
    }

    try {
        // Use cached constructor
        const pdf = new _jsPDF();
        console.log("pdf.svg exists:", typeof pdf.svg === 'function');

        if (typeof pdf.svg !== 'function') {
            // Fallback: try window.svg2pdf.svg2pdf directly
            if (window.svg2pdf && typeof window.svg2pdf.svg2pdf === 'function') {
                console.log("Using window.svg2pdf.svg2pdf()");
                await window.svg2pdf.svg2pdf(svg, pdf, { x, y, width, height });
            } else {
                console.error("No svg method available");
                return null;
            }
        } else {
            console.log("Using pdf.svg()");
            await pdf.svg(svg, { x, y, width, height });
        }

        console.log("Vector SVG added successfully");
        return pdf;
    } catch (e) {
        console.error("Error:", e);
        return null;
    }
};

/**
 * Generate QR code as SVG string
 *
 * Uses qrcode-generator library for browser-compatible QR code generation.
 * The QR code encodes a compact binary URL that captures all frame settings,
 * allowing the design to be recreated by scanning or visiting the URL.
 *
 * @param {string} url - The shareable URL to encode (81 chars max)
 * @returns {Promise<string>} - SVG string of the QR code
 */
window.generateQrCodeSvg = async function(url) {
    try {
        // Wait for library to load (important on mobile with slower connections)
        const loaded = await waitForQrCodeLibrary();
        if (!loaded) {
            console.error("qrcode library not loaded - check if CDN script loaded correctly");
            console.log("Available globals:", Object.keys(window).filter(k => k.toLowerCase().includes('qr')));
            return null;
        }

        // qrcode-generator: type 0 = auto-detect size, 'M' = medium error correction
        const qr = qrcode(0, 'M');
        qr.addData(url);
        qr.make();
        const svgString = qr.createSvgTag({ cellSize: 4, margin: 4 });
        console.log("QR code SVG generated successfully");
        return svgString;
    } catch (e) {
        console.error("Error generating QR code SVG:", e);
        return null;
    }
};

/**
 * Wait for qrcode library to load (with timeout)
 * @param {number} timeoutMs - Maximum time to wait in milliseconds
 * @returns {Promise<boolean>} - True if loaded, false if timeout
 */
async function waitForQrCodeLibrary(timeoutMs = 5000) {
    const startTime = Date.now();
    while (typeof qrcode === 'undefined') {
        if (Date.now() - startTime > timeoutMs) {
            console.error("Timeout waiting for qrcode library");
            return false;
        }
        // Wait 50ms before checking again
        await new Promise(resolve => setTimeout(resolve, 50));
    }
    return true;
}

/**
 * Generate QR code as PNG data URL for PDF embedding
 *
 * Uses qrcode-generator library to create a data URL suitable for jsPDF.
 *
 * @param {string} url - The shareable URL to encode
 * @returns {Promise<string>} - PNG data URL of the QR code
 */
window.generateQrCodeDataUrl = async function(url) {
    try {
        // Wait for library to load (important on mobile with slower connections)
        const loaded = await waitForQrCodeLibrary();
        if (!loaded) {
            console.error("qrcode library not loaded - check if CDN script loaded correctly");
            console.log("Available globals:", Object.keys(window).filter(k => k.toLowerCase().includes('qr')));
            return null;
        }

        // qrcode-generator: type 0 = auto-detect size, 'M' = medium error correction
        const qr = qrcode(0, 'M');
        qr.addData(url);
        qr.make();
        // createDataURL returns a base64 data URL (default is GIF, but works for PDF)
        // cellSize controls resolution - larger = higher res
        const dataUrl = qr.createDataURL(10, 4);  // cellSize=10, margin=4
        console.log("QR code data URL generated successfully");
        return dataUrl;
    } catch (e) {
        console.error("Error generating QR code data URL:", e);
        return null;
    }
};

/**
 * Instant settings restoration and UI initialization
 * Runs before PyScript loads for fast perceived startup
 */
document.addEventListener('DOMContentLoaded', () => {
    // Check for URL parameters first (takes priority over localStorage)
    const urlParams = new URLSearchParams(window.location.search);
    const configParam = urlParams.get('d');

    if (configParam) {
        // Decode binary config from URL
        const config = decodeFrameConfig(configParam);
        if (config) {
            applyConfigToForm(config, config.unit_mm);
            window.rfLoadedFromUrl = true;
            console.log('⚡ Settings loaded from URL parameter');

            // Clear URL params without reload (clean URL in address bar)
            window.history.replaceState({}, document.title, window.location.pathname);
        } else {
            console.error('Failed to decode URL config, falling back to localStorage');
        }
    }

    // Restore from localStorage if no URL params were used
    if (!window.rfLoadedFromUrl) {
        try {
            const settingsJson = localStorage.getItem('frame_designer_settings');
            if (settingsJson) {
                const settings = JSON.parse(settingsJson);

                // Restore all form fields
                const fields = [
                    'artwork-height', 'artwork-width', 'mat-width', 'frame-width',
                    'glazing-thickness', 'matboard-thickness', 'artwork-thickness',
                    'backing-thickness', 'rabbet-depth', 'frame-depth', 'blade-width'
                ];

                fields.forEach(fieldId => {
                    const element = document.getElementById(fieldId);
                    const settingKey = fieldId.replaceAll('-', '_');
                    if (element && settings[settingKey]) {
                        element.value = settings[settingKey];
                    }
                });

                console.log('⚡ Settings restored instantly via vanilla JS');
            }

            // Restore unit toggle button state
            const savedUnit = localStorage.getItem('frame_designer_unit');
            if (savedUnit === 'mm') {
                document.getElementById('unit-mm')?.classList.add('active');
                document.getElementById('unit-inches')?.classList.remove('active');
            }
        } catch (e) {
            console.error('Error in instant restore:', e);
        }
    }

    // Size selector tab switching
    const standardTab = document.getElementById('size-tab-standard');
    const customTab = document.getElementById('size-tab-custom');
    const standardPanel = document.getElementById('standard-sizes-panel');
    const customPanel = document.getElementById('custom-sizes-panel');

    if (standardTab && customTab) {
        standardTab.addEventListener('click', () => {
            standardTab.classList.add('active');
            customTab.classList.remove('active');
            standardPanel.style.display = 'block';
            customPanel.style.display = 'none';
        });

        customTab.addEventListener('click', () => {
            customTab.classList.add('active');
            standardTab.classList.remove('active');
            customPanel.style.display = 'block';
            standardPanel.style.display = 'none';

            // Pre-populate custom size fields with current artwork dimensions
            const heightInput = document.getElementById('artwork-height');
            const widthInput = document.getElementById('artwork-width');
            const customHeight = document.getElementById('custom-size-height');
            const customWidth = document.getElementById('custom-size-width');

            if (heightInput && widthInput && customHeight && customWidth) {
                customHeight.value = heightInput.value;
                customWidth.value = widthInput.value;
            }
        });
    }
});