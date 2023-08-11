let got_flamegraphs_data = false;

function getFlamegraphInfo(run, container_id) {
    var div = document.createElement('iframe');
    div.src = `data/js/${run}-flamegraph.svg`;
    div.style.width = `100%`;
    div.style.height = `100vh`;
    addElemToNode(container_id, div);
}

function flamegraphs() {
    if (got_flamegraphs_data) {
        return;
    }
    var data = runs_raw;
    var float_style = "none";
    if (data.length > 1) {
        float_style = "left";
    }
    var run_width = 100 / data.length;
    clearElements('flamegraphs-runs');
    data.forEach(function (value, index, arr) {
        // Run div
        var run_div = document.createElement('div');
        let this_run_data;
        run_div.id = `${value}-flamegraphs`;
        run_div.style.float = float_style;
        run_div.style.width = `${run_width}%`;
        addElemToNode('flamegraphs-runs', run_div);
        var run_node_id = run_div.id;

        // Run name
        var h3_run_name = document.createElement('h3');
        h3_run_name.innerHTML = value;
        h3_run_name.style.textAlign = "center";
        addElemToNode(run_node_id, h3_run_name);

        // Show data
        var per_run_div = document.createElement('div');
        per_run_div.id = `${value}-flamegraphs-run`;
        addElemToNode(run_node_id, per_run_div);
        for (let i = 0; i < flamegraph_raw_data['runs'].length; i++) {
            if (flamegraph_raw_data['runs'][i]['name'] == value) {
                setTimeout(() => {
                    getFlamegraphInfo(value, per_run_div.id);
                }, 0);
            }
        }
    })
    got_flamegraphs_data = true;
}
