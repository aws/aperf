import { clearElements, addElemToNode } from "./index.js";
export { kernelConfig };

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

function createEntries(container_id, values, level, run, diff) {
    values?.forEach(function(value, index, arr) {
        for (var prop in value) {
            if ('value' in value[prop]) {
                if (diff && !containsEntry(value[prop].name)) {
                    continue;
                }
                var dt = document.createElement('dt');
                dt.style.textIndent = `${level * 5}%`;
                dt.style.fontWeight = "normal";
                dt.innerHTML = `${value[prop].name} = ${value[prop].value}`;
                addElemToNode(container_id, dt);
                if (!diff) {
                    var entry = new Entry();
                    entry.run = run;
                    entry.title = value[prop].name;
                    entry.value = value[prop].value;
                    addEntry(entry);
                }
            } else {
                var dl = document.createElement('dl');
                dl.style.textIndent = `${level * 5}%`;
                dl.innerHTML = value[prop].name;
                dl.style.fontWeight = "bold";
                dl.id = `${run}-${value[prop].name}`;
                addElemToNode(container_id, dl);
                createEntries(dl.id, value[prop].entries, level + 1, run, diff);
            }
        }
    });
}

function getKernelConfig(run, container_id, diff) {
    const http = new XMLHttpRequest();
    http.onload = function () {
        var data = JSON.parse(http.response);
        var dl = document.createElement('dl');
        dl.id = `${run}-kernel-config`;
        dl.style.float = "none";
        var dl_id = dl.id;
        addElemToNode(container_id, dl);
        data.forEach(function (value, index, arr) {
            var dt = document.createElement('dl');
            dt.id = `${run}-${value.name}`;
            dt.style.fontWeight = "bold";
            dt.innerHTML = value.name;
            addElemToNode(dl_id, dt);
            createEntries(dt.id, value.entries, 1, run, diff);
        });
    }
    http.open("GET", `visualize/kernel_config?run=${run}&get=values`);
    http.send();
}

function kernelConfig(diff: boolean) {
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
        clearElements('kernel-config-runs');
        data.forEach(function (value, index, arr) {
            // Run div
            var run_div = document.createElement('div');
            run_div.id = `${value}-kernel-config`;
            run_div.style.float = float_style;
            run_div.style.width = `${run_width}%`;
            addElemToNode('kernel-config-runs', run_div);
            var run_node_id = run_div.id;

            // Run name
            var h3_run_name = document.createElement('h3');
            h3_run_name.innerHTML = `${value}`;
            h3_run_name.style.textAlign = "center";
            addElemToNode(run_node_id, h3_run_name);

            //Show aggregate data
            var agg_elem = document.createElement('div');
            agg_elem.id = `${value}-kernel-config-div`;
            addElemToNode(run_node_id, agg_elem);
            getKernelConfig(value, agg_elem.id, diff);
        })
    }
    http.open("GET", 'visualize/get?get=entries');
    http.send();
}
