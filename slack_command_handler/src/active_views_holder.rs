use std::{
    collections::{
        HashMap
    },
    sync::{
        Mutex
    }
};
use crate::{
    slack::{
        ViewActionHandler
    }
};

#[derive(Default)]
pub struct ViewsHandlersHolder{
    active_views: Mutex<HashMap<String, Box<dyn ViewActionHandler + Send>>>
}

impl ViewsHandlersHolder{
    pub fn push_view_handler(&self, view_handler: Box<dyn ViewActionHandler + Send>){
        if let Ok(mut guard) = self.active_views.lock(){
            guard.insert(view_handler.get_view().get_id().to_owned(), view_handler);
        }
    }
    
    pub fn pop_view_handler(&self, id: &str) -> Option<Box<dyn ViewActionHandler + Send>>{
        if let Ok(mut guard) = self.active_views.lock(){
            guard.remove(id)
        }else{
            None
        }
    }
}

