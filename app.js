/**
 * ReferenceFrame - JavaScript initialization
 *
 * This file handles:
 * - PDF export with vector SVG (jsPDF helper)
 * - Instant settings restoration from localStorage
 * - UI tab switching before PyScript loads
 */

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
 * Instant settings restoration and UI initialization
 * Runs before PyScript loads for fast perceived startup
 */
document.addEventListener('DOMContentLoaded', () => {
    // Restore form values immediately from localStorage (< 100ms)
    // This provides instant UI restoration while PyScript boots up in background
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