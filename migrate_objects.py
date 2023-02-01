#!/usr/bin/env python3

from dotenv import load_dotenv

load_dotenv()
foodshare_project_ref = os.getenv('foodshare_project_ref')
postgres_project_ref = os.getenv('postgres_project_ref')
foodshare_service_key = os.getenv('foodshare_service_key')
postgres_service_key = os.getenv('postgres_service_key')


#Edit here:
OLD_DB_URL='https://foodshare_project_ref.supabase.co'
NEW_DB_URL='https://postgres_project_ref.supabase.co'
OLD_SERVICE_KEY='foodshare_service_key'
NEW_SERVICE_KEY='postgres_service_key'

# Script:
from supabase import create_client
import os
filedata = ''

#creating the clients for the old & new projects
old_supabase_client = create_client(OLD_DB_URL, OLD_SERVICE_KEY)
new_supabase_client = create_client(NEW_DB_URL, NEW_SERVICE_KEY)

#Create all buckets
buckets = old_supabase_client.storage().list_buckets()
for bucket in buckets:
    print("Copying objects from "+bucket.name)
    objects = old_supabase_client.storage().from_(bucket.name).list()
    try:
      new_supabase_client.storage().create_bucket(bucket.name, public=bucket.public)
    except:
      print("unable to create bucket")
    for obj in objects:
        print(obj['name'])
        try:
          with open(obj['name'], 'wb+') as f:
            res = old_supabase_client.storage().from_(bucket.name).download(obj['name'])
            f.write(res)
            f.close()
        except Exception as e: 
            print("error downloading "+ str(e))
        try:
          with open(obj['name'], 'rb+') as f:
            res = new_supabase_client.storage().from_(bucket.name).upload(obj['name'], obj['name'])
          # Delete file after uploading it
          if os.path.exists(os.path.abspath(obj['name'])):
              os.remove(os.path.abspath(obj['name']))
        except Exception as e: 
          print("error uploading | " + str(e))
