let got_flamegraphs_data: boolean|string = "none";

function getJavaFlamegraphInfo(run, container_id, run_data){
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
        div.src = `data/js/${run}-java-flamegraph-${key}.html`;
        div.style.width = `100%`;
        div.style.height = `100vh`;
        addElemToNode(container_id, div);
    }
}

function getFlamegraphInfo(run, container_id) {
    var div = document.createElement('iframe');
    div.src = `data/js/${run}-flamegraph.svg`;
    div.style.width = `100%`;
    div.style.height = `100vh`;
    addElemToNode(container_id, div);
}

function flamegraphs(set) {
    if (set == got_flamegraphs_data) {
        return;
    }
    got_flamegraphs_data = set;
    clear_and_create('flamegraphs');
    let raw_data = (set == 'flamegraphs') ? flamegraph_raw_data : java_profile_raw_data;
    for (let i = 0; i < raw_data['runs'].length; i++) {
        let run_name = raw_data['runs'][i]['name'];
        let elem_id = `${run_name}-flamegraphs-per-data`;
        setTimeout(() => {
            switch(set){
                case 'flamegraphs':
                    getFlamegraphInfo(run_name, elem_id);
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
