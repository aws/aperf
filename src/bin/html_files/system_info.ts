import './plotly.js';
import { clearElements, addElemToNode } from './index.js';
export { systemInfo };

function getSystemInfo(run, container_id) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        data.forEach(function (value, index, arr) {
            var div = document.createElement('div');
            div.id = `${run}-${value.name}-container`;
            addElemToNode(container_id, div);
            var b = document.createElement('b');
            b.style.display = "inline-block";
            b.innerHTML = `${value.name}: `;
            addElemToNode(div.id, b);
            var text_value = document.createElement('div')
            text_value.style.display = "inline-block";
            text_value.innerHTML = ` ${value.value}`;
            addElemToNode(div.id, text_value);
            var p = document.createElement('p');
            addElemToNode(div.id, p);
        })
    }
    http.open("GET", `visualize/system_info?run=${run}&get=values`);
    http.send();
}

function systemInfo() {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('system-info-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-system-info`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('system-info-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = value;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            // Show data
            var per_run_div = document.createElement('div');
            per_run_div.id = `${value}-system-info-run`;
            addElemToNode(run_node_id, per_run_div);
            getSystemInfo(value, per_run_div.id);
        })
    }
    http.open("GET", '/visualize/get?get=entries');
    http.send();
}
