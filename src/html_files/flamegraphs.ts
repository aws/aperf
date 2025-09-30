let got_flamegraphs_data: boolean|string = "none";

function getJavaFlamegraphInfo(run, container_id, run_data, metric){
    if (handleNoData(container_id, run_data)) return;

    let values = JSON.parse(run_data['values']);
    let data = values.find((d) => d['data_type'] == metric);

    let sorted = data['graphs'].filter((graph) => !graph["graph_name"].includes('-')).toSorted((x, y) => y["graph_size"] - x["graph_size"]);

    if(sorted.length == 0){
        var h3 = document.createElement('h3');
        h3.innerText = `No data collected.`;
        addElemToNode(container_id, h3);
    }

    for(let graph of sorted){
        var h3 = document.createElement('h3');
        h3.style.textAlign = "center";
        h3.innerText = graph["graph_name"];

        addElemToNode(container_id, h3);

        var div = document.createElement('iframe');
        div.src = graph["graph_path"];
        div.style.width = `100%`;
        div.style.height = `100vh`;
        addElemToNode(container_id, div);
    }
}

function getFlamegraphInfo(run, container_id, run_data) {
    if (handleNoData(container_id, run_data)) return;

    var div = document.createElement('iframe');
    div.src = run_data['values'];
    div.style.width = `100%`;
    div.style.height = `100vh`;
    addElemToNode(container_id, div);

    // Reverse flamegraph
    let reverse_path = run_data['values'].replace('-flamegraph.svg', '-reverse-flamegraph.svg');
    var h3 = document.createElement('h3');
    h3.innerText = 'Reverse Flamegraph';
    h3.style.textAlign = 'center';
    addElemToNode(container_id, h3);
    var reverse_div = document.createElement('iframe');
    reverse_div.src = reverse_path;
    reverse_div.style.width = `100%`;
    reverse_div.style.height = `100vh`;
    addElemToNode(container_id, reverse_div);
}

function flamegraphs(set) {
    if (set == got_flamegraphs_data) {
        return;
    }
    got_flamegraphs_data = set;
    clear_and_create('flamegraphs');
    let raw_data = (set == 'flamegraphs') ? flamegraphs_raw_data : java_profile_raw_data;
    for (let i = 0; i < raw_data['runs'].length; i++) {
        let run_name = raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-flamegraphs-per-data`;
        setTimeout(() => {
            switch(set){
                case 'flamegraphs':
                    getFlamegraphInfo(run_name, elem_id, raw_data['runs'][i]['key_values']);
                    break;
                case 'javaprofile-cpu':
                    getJavaFlamegraphInfo(run_name, elem_id, raw_data['runs'][i]['key_values'], 'cpu');
                    break;
                case 'javaprofile-alloc':
                    getJavaFlamegraphInfo(run_name, elem_id, raw_data['runs'][i]['key_values'], 'alloc');
                    break;
                case 'javaprofile-wall':
                    getJavaFlamegraphInfo(run_name, elem_id, raw_data['runs'][i]['key_values'], 'wall');
                    break;
                case 'javaprofile-legacy':
                    getJavaFlamegraphInfo(run_name, elem_id, raw_data['runs'][i]['key_values'], 'legacy');
                    break;
                default:
                    return;
            }
        }, 0);
    }
}
