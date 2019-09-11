$(document).on('click', '[data-toggle="lightbox"]', function(event) {
	event.preventDefault();
	$(this).ekkoLightbox({alwaysShowClose: true});
});

window.addEventListener("load", function() {
	$(document).ready(function() {
		$("#showPassword").change(function() {
			if (this.checked) {
				$('#password').attr('type', 'text');
			} else {
				$('#password').attr('type', 'password');
			}
		});
	});
});
