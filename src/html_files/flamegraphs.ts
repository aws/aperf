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
    clear_and_create('flamegraphs');
    for (let i = 0; i < flamegraph_raw_data['runs'].length; i++) {
        let run_name = flamegraph_raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-flamegraphs-per-data`;
        setTimeout(() => {
            getFlamegraphInfo(run_name, elem_id);
        }, 0);
    }
    got_flamegraphs_data = true;
}
