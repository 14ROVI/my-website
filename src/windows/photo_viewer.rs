use yew::{function_component, html, use_state, Callback, Html};

#[function_component(PhotoViewer)]
pub fn photo_viewer() -> Html {
    let images = vec![
        "000005220003.jpg",
        "000005220005.jpg",
        "000006710020.jpg",
        "000006710032.jpg",
        "000101080016.png",
        "000101080027.png",
        "000101080035.png",
        "000156840002.jpg",
        "000156840009.jpg",
        "A028792-R1-19-18A.JPG",
        "cloud.jpg",
        "PUNCH.png",
    ];

    let photo_id = use_state(|| 0);
    let decrement = {
        let photo_id = photo_id.clone();
        let new_photo_id = if *photo_id == 0 {
            images.len() - 1
        } else {
            *photo_id - 1
        };
        Callback::from(move |_| photo_id.set(new_photo_id))
    };
    let increment = {
        let photo_id = photo_id.clone();
        let new_photo_id = if *photo_id == images.len() - 1 {
            0
        } else {
            *photo_id + 1
        };
        Callback::from(move |_| photo_id.set(new_photo_id))
    };

    let photo = images[*photo_id];

    html! {
        <div class="pp">
        <div class="photo-window">
            <div class="photo-path-container">
                // current photo
                <div class="photo-path">
                <span>{format!("C:/home/roan/photos/{}", photo)}</span><br/>
                <img alt="Cloud" src={format!("assets/photo_gallery/{}", photo)}/>
                </div>
            </div>
            <div class="photo-controls">
                <button onclick={decrement}>{" < "}</button>
                <button onclick={increment}>{" > "}</button>
            </div>
            <div>
            <div class="photo-selector-container">
                <div class="photo-selector">
                    // photos
                    {
                        images.into_iter().enumerate().map(|(i, image)| html! {
                            <img class={(i==*photo_id).then_some("selected")} src={format!("assets/photo_gallery/{}", image)}/>
                        }).collect::<Html>()
                    }
                </div>
            </div>
            </div>
        </div>
        </div>
    }
}
