import './plotly.js';
import { clearElements, addElemToNode } from './index.js';
export { diskStats };

function getStatValues(run, key, elem, unit) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var disk_datas = [];
        var data = JSON.parse(http.response);
        data.forEach(function (v, i, a) {
            var x_time = [];
            var y_data = [];
            v.values.forEach(function(disk_value, disk_index, disk_arr){
                x_time.push(disk_value.time.TimeDiff);
                y_data.push(disk_value.value);
            })
            var disk_data = {
                name: v.name,
                x: x_time,
                y: y_data,
                type: 'scatter',
            };
            disk_datas.push(disk_data);
        })
        var TESTER = elem;
        var layout = {
            title: key.name,
            xaxis: {
                title: 'Time (s)',
            },
            yaxis: {
                title: key.unit,
            },
        };
        Plotly.newPlot(TESTER, disk_datas, layout, { frameMargins: 0 });
    }
    http.open("GET", `visualize/disk_stats?run=${run}&get=values&key=${key.name}&unit=${unit}`);
    http.send();
}

function getStatKeys(run, container_id, mb) {
    var unit = "KB";
    if (mb) {
	unit = "MB";
    }
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        data.forEach(function (value, index, arr) {
            var elem = document.createElement('div');
            elem.id = `disk-stat-${run}-${value.name}`;
            elem.style.float = "none";
            addElemToNode(container_id, elem);
            getStatValues(run, value, elem, unit);
        })
    }
    http.open("GET", `visualize/disk_stats?run=${run}&get=keys&unit=${unit}`);
    http.send();
}

function diskStats(mb: boolean) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('disk-stat-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-disk-stat`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('disk-stat-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = value;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            // Show data
            var per_value_div = document.createElement('div');
            per_value_div.id = `${value}-disk-stat-per-data`;
            addElemToNode(run_node_id, per_value_div);
            getStatKeys(value, per_value_div.id, mb);
        })
    }
    http.open("GET", '/visualize/get?get=entries');
    http.send();
}
