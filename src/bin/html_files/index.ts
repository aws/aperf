function openData(evt: Event, tabName: string) {
	var tabcontent = Array.from(document.getElementsByClassName('tabcontent') as HTMLCollectionOf<HTMLElement>);
	var tablinks = Array.from(document.getElementsByClassName("tablinks") as HTMLCollectionOf<HTMLElement>);
	tabcontent.forEach((element) => {
		element.style.display = "none";
	});
	tablinks.forEach((element) => {
		element.className.replace(" active", "");
	})
	document.getElementById(tabName).style.display = "block";
	const currentTarget = evt.currentTarget as HTMLButtonElement;
	currentTarget.className += " active";
}
// Collapse functionality
var coll = Array.from(document.getElementsByClassName("collapsible") as HTMLCollectionOf<HTMLElement>);
coll.forEach((element) => {
	element.addEventListener("click", function () {
		this.classList.toggle("active");
		var content = this.nextElementSibling as HTMLElement;
		if (content.style.display === "block") {
			content.style.display = "none";
		} else {
			content.style.display = "block";
		}
	})
})
