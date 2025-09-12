let got_flamegraphs_data: boolean|string = "none";

function getJavaFlamegraphInfo(run, container_id, run_data){
    if (handleNoData(container_id, run_data)) return;

    let data = JSON.parse(run_data['values']);

    let sorted = Object.keys(data).sort(function(x,y){
        return data[y][1] - data[x][1];
    });

    if(sorted.length == 0){
        var h3 = document.createElement('h3');
        h3.innerText = `No data collected.`;
        addElemToNode(container_id, h3);
    }

    for(let key of sorted){
        let value = data[key][0];
        var h3 = document.createElement('h3');
        h3.style.textAlign = "center";
        h3.innerText = `JVM: ${value}, PID: ${key}`;
        addElemToNode(container_id, h3);
        var div = document.createElement('iframe');
        div.src = `data/js/${run}-java-profile-${key}.html`;
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
                case 'javaprofile':
                    getJavaFlamegraphInfo(run_name, elem_id, raw_data['runs'][i]['key_values']);
                    break;
                default:
                    return;
            }
        }, 0);
    }
}
