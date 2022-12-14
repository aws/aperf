import { clearElements, addElemToNode } from "./index.js";
export { sysctl };

var diff_data: Array<Entry> = [];
var common_data: Array<Entry> = [];
class Entry {
    run: string;
    title: string;
    value: string;
}

function addEntry(entry: Entry) {
    var add = true;
    for (let i = 0; i < common_data.length; i++) {
        if (common_data[i].title == entry.title &&
            common_data[i].value == entry.value) {
                return;
            }
    }
    for (let i = 0; i < diff_data.length; i++) {
        if (diff_data[i].title == entry.title &&
            diff_data[i].value == entry.value) {
                common_data.push(entry);
                diff_data.splice(i, 1);
                return;
            }
    }
    diff_data.push(entry);
}

function containsEntry(title: string) {
    var found = false;
    diff_data.forEach(function (value, index, arr) {
        if (value.title == title) {
            found = true;
        }
    })
    return found;
}

function getSysctlData(run, container_id, diff) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var dl = document.createElement('dl');
        dl.id = `${run}-sysctl-data`;
        dl.style.float = "none";
        var dl_id = dl.id;
        addElemToNode(container_id, dl);
        for (var key in data) {
            var value = data[key];
            if (diff && !containsEntry(key)) {
                continue;
            }
            var dt = document.createElement('dt');
            dt.innerHTML = `${key} = ${value}`;
            if (!diff) {
                var entry = new Entry();
                entry.run = run;
                entry.title = key;
                entry.value = String(value);
                addEntry(entry);
            }
            addElemToNode(dl.id, dt);
        }
    }
    http.open("GET", `visualize/sysctl?run=${run}&get=values`);
    http.send();
}

function sysctl(diff: boolean) {
    if (!diff) {
        diff_data = [];
    }
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var float_style = "none";
        if (data.length > 1) {
            float_style = "left";
        }
        var run_width = 100 / data.length;
        clearElements('sysctl-data-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-sysctl-data`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('sysctl-data-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = `${value}`;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            //Show aggregate data
            var agg_elem = document.createElement('div');
            agg_elem.id = `${value}-sysctl-data-div`;
            addElemToNode(run_node_id, agg_elem);
            getSysctlData(value, agg_elem.id, diff);
        })
    }
    http.open("GET", 'visualize/get?get=entries');
    http.send();
}
